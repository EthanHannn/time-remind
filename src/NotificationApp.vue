<script setup lang="ts">
import type { NotificationPreviewRequest } from './types/notificationPreview'
import type { Reminder } from './types/reminder'
import type { MascotStyle } from './utils/mascotStyles'
import type { NotificationAppearance } from './utils/notificationAppearance'
import type { NotificationSoundPreset } from './utils/notificationSound'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import NotificationCard from './components/NotificationCard.vue'
import { useTheme } from './composables/useTheme'
import { loadLanguage, useI18n } from './i18n'
import { normalizeMascotStyle } from './utils/mascotStyles'
import { normalizeNotificationAppearance } from './utils/notificationAppearance'
import { playNotificationSound } from './utils/notificationSound'
import { getLocalizedReminderVisual } from './utils/reminderVisuals'

const props = defineProps<{
  preview?: boolean
  previewRequest?: NotificationPreviewRequest
}>()

const emit = defineEmits<{ closed: [] }>()

const { applyTheme } = useTheme()

const mascotStyle = shallowRef<MascotStyle>('classic')
const notificationAppearance = shallowRef<NotificationAppearance>('soft')
const name = ref('')
const message = ref('')
const reminderId = ref('')
const notificationId = ref('')
const reminderType = ref('custom')
const reminderIcon = ref('custom_star')
const actionEnabled = ref(false)
const actionTitle = ref('')
const actionMessage = ref('')
const actionDurationSeconds = ref(0)
const visible = ref(false)
const notificationDuration = ref(30000)
const soundEnabled = ref(true)
const soundPreset = ref<NotificationSoundPreset>('soft')
const soundVolume = ref(60)
const breakMode = ref(false)
const collapsed = shallowRef(false)
const interacting = shallowRef(false)
const breakRemainingSeconds = ref(0)
const breakCountdownRunning = shallowRef(false)
const pendingCount = ref(0)
const { locale, t, formatError, unit, clock } = useI18n()

interface SettingsPayload {
  theme?: string
  mascotStyle?: MascotStyle
  notificationAppearance?: NotificationAppearance
  notificationDuration?: number
  postponeOptions?: number[]
  soundEnabled?: boolean
  soundPreset?: NotificationSoundPreset
  soundVolume?: number
}

interface NotificationOption {
  label: string
  minutes: number
}

interface NotificationPayload {
  queue_revision: number
  notification_id?: string
  reminder_id?: string
  name?: string
  message?: string
  reminder_type?: string
  icon?: string
  break_duration_minutes?: number
  break_notification_enabled?: boolean
  action_enabled?: boolean
  action_title?: string
  action_message?: string
  action_duration_seconds?: number
  action_completion_mode?: 'auto' | 'manual'
  pending_count?: number
}

interface NotificationQueuePayload {
  queue_revision: number
  current_notification_id?: string | null
  current_reminder_id?: string | null
  pending_count?: number
}

const visual = computed(() => getLocalizedReminderVisual(reminderType.value, locale.value, reminderIcon.value, mascotStyle.value))

const breakCountdownLabel = computed(() => clock(breakRemainingSeconds.value, true))

const pendingLabel = computed(() => {
  const count = Math.max(0, pendingCount.value)
  return count > 0 ? t('notification.pendingCount', { count }) : ''
})

function buildPostponeOptions(values?: number[]): NotificationOption[] {
  const source = values && values.length > 0 ? values : [5, 10, 15]
  return source.map(minutes => ({
    label: unit(minutes, 'minute'),
    minutes,
  }))
}

const postponeOptions = ref<NotificationOption[]>(buildPostponeOptions())

function normalizeSoundPreset(value?: string): NotificationSoundPreset {
  return value === 'bright' || value === 'calm' || value === 'anime' || value === 'arcade' ? value : 'soft'
}

function normalizeSoundVolume(value?: number): number {
  const safe = Number.isFinite(value) ? Number(value) : 60
  return Math.min(Math.max(Math.round(safe), 0), 100)
}

function getLegacySettings(): SettingsPayload | null {
  const saved = localStorage.getItem('app-settings')
  if (!saved)
    return null

  try {
    return JSON.parse(saved) as SettingsPayload
  }
  catch {
    return null
  }
}

async function loadDisplaySettings() {
  try {
    const settings = await invoke<Record<string, string>>('get_all_settings')
    mascotStyle.value = normalizeMascotStyle(settings.mascot_style)
    notificationAppearance.value = normalizeNotificationAppearance(settings.notification_appearance)
    notificationDuration.value = Math.min(Math.max(Number(settings.notification_duration || 30), 5), 120) * 1000
    soundEnabled.value = settings.sound_enabled !== 'false'
    soundPreset.value = normalizeSoundPreset(settings.sound_preset)
    soundVolume.value = normalizeSoundVolume(Number(settings.sound_volume || 60))
    await loadLanguage(settings)
    postponeOptions.value = buildPostponeOptions(settings.postpone_options ? JSON.parse(settings.postpone_options) as number[] : undefined)
    applyTheme(settings.theme || 'system')
  }
  catch {
    await loadLanguage()
    const legacy = getLegacySettings()
    mascotStyle.value = normalizeMascotStyle(legacy?.mascotStyle)
    notificationAppearance.value = normalizeNotificationAppearance(legacy?.notificationAppearance)
    notificationDuration.value = Math.min(Math.max(legacy?.notificationDuration || 30, 5), 120) * 1000
    soundEnabled.value = legacy?.soundEnabled ?? true
    soundPreset.value = normalizeSoundPreset(legacy?.soundPreset)
    soundVolume.value = normalizeSoundVolume(legacy?.soundVolume)
    postponeOptions.value = buildPostponeOptions(legacy?.postponeOptions)
    applyTheme(legacy?.theme || 'system')
  }
}

let autoDismissTimer: ReturnType<typeof setTimeout> | null = null
let autoDismissDeadline: number | null = null
let autoDismissRemainingMs: number | null = null
let breakTimer: ReturnType<typeof setInterval> | null = null
let unlistenShow: (() => void) | null = null
let unlistenQueueUpdated: (() => void) | null = null
let unlistenSystemPaused: (() => void) | null = null
let unlistenSettings: (() => void) | null = null
let unlistenSystemResumed: (() => void) | null = null
let systemTimersPaused = false
let latestQueueRevision = -1
let notificationRevision = 0
const actionPending = ref(false)
const actionError = ref('')
let disposed = false
const baseHeight = computed(() => notificationAppearance.value === 'illustrated' ? 264 : notificationAppearance.value === 'minimal' ? 232 : 240)
const compact = computed(() => collapsed.value && breakMode.value)
const actionAvailable = computed(() => actionEnabled.value && actionDurationSeconds.value > 0)
const contentHeight = computed(() => (compact.value ? 96 : baseHeight.value + (breakMode.value ? 32 : 0)) + (actionError.value ? 44 : 0))
// Reserve transparent space above action reminders before they appear. Starting
// an action expands only the card, keeping the native window and its anchor still.
const windowSize = computed(() => ({
  width: compact.value ? 340 : 400,
  height: (compact.value ? 96 : baseHeight.value + (actionAvailable.value ? 32 : 0)) + (actionError.value ? 44 : 0),
}))
let resizeQueue = Promise.resolve()
watch([visible, () => windowSize.value.width, () => windowSize.value.height], () => {
  if (!visible.value || props.previewRequest)
    return
  resizeQueue = resizeQueue.then(async () => {
    if (!disposed && visible.value)
      await invoke('resize_notification_window', windowSize.value)
  }).catch(error => console.error('Failed to resize notification:', error))
})
function setInteracting(active: boolean) {
  interacting.value = active
  if (active)
    pauseAutoDismiss()
  else if (!systemTimersPaused)
    resumeAutoDismiss()
}
let breakDeadline = 0

async function showPreview(request: NotificationPreviewRequest) {
  const revision = ++notificationRevision
  clearAutoDismiss()
  stopBreakCountdown()
  visible.value = false
  await loadLanguage({ language: request.settings.language || locale.value })
  if (disposed || revision !== notificationRevision)
    return

  mascotStyle.value = normalizeMascotStyle(request.mascotStyle)
  notificationAppearance.value = normalizeNotificationAppearance(request.settings.notificationAppearance)
  reminderType.value = request.reminderType
  reminderIcon.value = request.icon || request.reminderType
  applyTheme(request.settings.theme)
  notificationDuration.value = Math.min(Math.max(request.settings.notificationDuration || 30, 5), 120) * 1000
  postponeOptions.value = buildPostponeOptions(request.settings.postponeOptions)
  name.value = visual.value.defaultName
  message.value = visual.value.shortMessage
  actionEnabled.value = request.reminderType === 'rest' || request.reminderType === 'eye_care'
  actionDurationSeconds.value = visual.value.defaultActionDurationSeconds || 0
  actionTitle.value = ''
  actionMessage.value = ''
  actionPending.value = false
  breakMode.value = false
  collapsed.value = false
  pendingCount.value = 0
  visible.value = true
  if (request.settings.soundEnabled !== false) {
    playNotificationSound({
      preset: normalizeSoundPreset(request.settings.soundPreset),
      volume: normalizeSoundVolume(request.settings.soundVolume),
    })
  }
  startAutoDismiss()
}

watch(() => props.previewRequest, (request) => {
  if (props.preview && request)
    void showPreview(request)
}, { immediate: true })

function handlePreviewKeydown(event: KeyboardEvent) {
  if (props.preview && event.key === 'Escape')
    closeNotificationWindow()
}

async function refreshReminderCopy() {
  const id = reminderId.value
  const revision = notificationRevision
  if (!visible.value || !id)
    return
  try {
    const reminders = await invoke<Reminder[]>('get_reminders')
    if (disposed || revision !== notificationRevision || id !== reminderId.value)
      return
    const reminder = reminders.find(item => item.id === id)
    if (reminder) {
      name.value = reminder.name
      message.value = reminder.message
      actionTitle.value = reminder.action_title
      actionMessage.value = reminder.action_message
    }
  }
  catch (error) {
    console.error('Failed to refresh reminder translations:', error)
  }
}

onMounted(async () => {
  if (props.preview) {
    window.addEventListener('keydown', handlePreviewKeydown)
    if (props.previewRequest)
      return

    const appWindow = getCurrentWebviewWindow()
    const stop = await appWindow.listen<NotificationPreviewRequest>('notification:preview', (event) => {
      void showPreview(event.payload)
    })
    if (disposed) {
      stop()
      return
    }
    unlistenShow = stop
    const revision = notificationRevision
    const initial = await invoke<NotificationPreviewRequest | null>('get_notification_preview')
    if (initial && !disposed && revision === notificationRevision)
      await showPreview(initial)
    return
  }
  const appWindow = getCurrentWebviewWindow()
  await loadDisplaySettings()
  unlistenSettings = await appWindow.listen('settings:changed', () => {
    void loadDisplaySettings().then(refreshReminderCopy)
  })

  unlistenShow = await appWindow.listen<NotificationPayload>('notification:show', (event) => {
    const data = event.payload
    if (!data?.notification_id || !data?.reminder_id || !data?.name) {
      return
    }

    if (data.queue_revision < latestQueueRevision)
      return
    latestQueueRevision = data.queue_revision

    notificationRevision += 1
    actionPending.value = false
    actionError.value = ''
    clearAutoDismiss()
    stopBreakCountdown()
    breakMode.value = false
    collapsed.value = false
    name.value = data.name
    message.value = data.message || ''
    reminderId.value = data.reminder_id
    notificationId.value = data.notification_id
    reminderType.value = data.reminder_type || 'custom'
    reminderIcon.value = data.icon || data.reminder_type || 'custom_star'
    actionEnabled.value = Boolean(data.action_enabled)
    actionTitle.value = data.action_title || ''
    actionMessage.value = data.action_message || ''
    actionDurationSeconds.value = data.action_duration_seconds || 0
    pendingCount.value = Math.max(0, Number(data.pending_count || 0))
    visible.value = true

    const shownRevision = notificationRevision
    void loadDisplaySettings().then(() => {
      if (!visible.value || breakMode.value || notificationRevision !== shownRevision)
        return

      if (soundEnabled.value) {
        playNotificationSound({
          preset: soundPreset.value,
          volume: soundVolume.value,
        })
      }
      startAutoDismiss()
    })
  })

  unlistenQueueUpdated = await appWindow.listen<NotificationQueuePayload>('notification:queue-updated', (event) => {
    const data = event.payload
    if (!data || data.queue_revision < latestQueueRevision)
      return
    latestQueueRevision = data.queue_revision

    if (data.current_notification_id && data.current_notification_id !== notificationId.value)
      return

    if (data.current_reminder_id && data.current_reminder_id !== reminderId.value)
      return

    if (!data.current_reminder_id) {
      // Native hide does not stop JavaScript timers in this persistent webview.
      clearAutoDismiss()
      stopBreakCountdown()
      visible.value = false
      breakMode.value = false
      resetNotificationState()
      return
    }

    pendingCount.value = Math.max(0, Number(data.pending_count || 0))
  })

  unlistenSystemPaused = await appWindow.listen('system:paused', () => {
    pauseLocalTimers()
  })

  unlistenSystemResumed = await appWindow.listen('system:resumed', () => {
    resumeLocalTimers()
  })
})

onUnmounted(() => {
  disposed = true
  notificationRevision += 1
  window.removeEventListener('keydown', handlePreviewKeydown)
  unlistenShow?.()
  unlistenQueueUpdated?.()
  unlistenSystemPaused?.()
  unlistenSystemResumed?.()
  unlistenSettings?.()
  clearAutoDismiss()
  stopBreakCountdown()
})

function clearAutoDismiss() {
  if (autoDismissTimer) {
    clearTimeout(autoDismissTimer)
    autoDismissTimer = null
  }
  autoDismissDeadline = null
  autoDismissRemainingMs = null
}

function startAutoDismiss(durationMs = notificationDuration.value) {
  clearAutoDismiss()
  if (!visible.value || breakMode.value)
    return

  if (systemTimersPaused || interacting.value) {
    autoDismissRemainingMs = durationMs
    return
  }

  autoDismissDeadline = Date.now() + durationMs
  autoDismissTimer = setTimeout(() => {
    autoDismissTimer = null
    autoDismissDeadline = null
    autoDismissRemainingMs = null
    void handleAction('timeout')
  }, durationMs)
}

function pauseAutoDismiss() {
  if (!autoDismissTimer || autoDismissDeadline === null)
    return

  autoDismissRemainingMs = Math.max(0, autoDismissDeadline - Date.now())
  clearTimeout(autoDismissTimer)
  autoDismissTimer = null
  autoDismissDeadline = null
}

function resumeAutoDismiss() {
  if (autoDismissRemainingMs === null || !visible.value || breakMode.value || systemTimersPaused || interacting.value)
    return

  const remainingMs = autoDismissRemainingMs
  autoDismissRemainingMs = null
  if (remainingMs <= 0) {
    void handleAction('timeout')
    return
  }

  startAutoDismiss(remainingMs)
}

function stopBreakCountdown() {
  breakCountdownRunning.value = false
  if (breakTimer) {
    clearInterval(breakTimer)
    breakTimer = null
  }
}

function runBreakCountdown() {
  if (systemTimersPaused)
    return

  stopBreakCountdown()
  breakDeadline = Date.now() + breakRemainingSeconds.value * 1000
  breakCountdownRunning.value = true
  breakTimer = setInterval(() => {
    breakRemainingSeconds.value = Math.max(0, Math.ceil((breakDeadline - Date.now()) / 1000))
    if (breakRemainingSeconds.value <= 0) {
      stopBreakCountdown()
      void closeBreakPrompt(false)
    }
  }, 1000)
}

function startBreakCountdown(seconds = actionDurationSeconds.value) {
  stopBreakCountdown()
  breakMode.value = true
  breakRemainingSeconds.value = seconds
  runBreakCountdown()
}

function pauseLocalTimers() {
  if (systemTimersPaused)
    return

  systemTimersPaused = true
  pauseAutoDismiss()
  stopBreakCountdown()
}

async function resumeLocalTimers() {
  if (!systemTimersPaused)
    return

  systemTimersPaused = false
  if (breakMode.value) {
    if (!props.preview) {
      const revision = notificationRevision
      try {
        const progress = await invoke<{ remaining_seconds: number } | null>('get_action_progress', {
          reminderId: reminderId.value,
          notificationId: notificationId.value,
        })
        if (revision !== notificationRevision)
          return
        if (progress)
          breakRemainingSeconds.value = progress.remaining_seconds
      }
      catch (error) {
        if (revision !== notificationRevision)
          return
        actionError.value = t('feedback.failed', { error: formatError(error) })
      }
    }
    if (breakRemainingSeconds.value <= 0) {
      void closeBreakPrompt(false)
      return
    }

    runBreakCountdown()
    return
  }

  resumeAutoDismiss()
}

function resetNotificationState() {
  notificationRevision += 1
  actionPending.value = false
  actionError.value = ''
  name.value = ''
  message.value = ''
  reminderId.value = ''
  notificationId.value = ''
  reminderType.value = 'custom'
  actionEnabled.value = false
  actionTitle.value = ''
  actionMessage.value = ''
  actionDurationSeconds.value = 0
  breakRemainingSeconds.value = 0
  pendingCount.value = 0
}

function closeNotificationWindow() {
  visible.value = false
  breakMode.value = false
  collapsed.value = false
  interacting.value = false
  clearAutoDismiss()
  stopBreakCountdown()
  resetNotificationState()
  if (props.preview) {
    emit('closed')
    if (!props.previewRequest)
      void getCurrentWebviewWindow().hide().catch(console.error)
  }
}

async function closeBreakPrompt(finishBreakNow: boolean) {
  if (!visible.value || actionPending.value)
    return
  if (props.preview) {
    closeNotificationWindow()
    return
  }
  actionPending.value = true
  const revision = notificationRevision
  stopBreakCountdown()
  actionError.value = ''

  try {
    await invoke('release_notification', {
      notificationId: notificationId.value,
      reminderId: reminderId.value,
      finishBreakNow,
    })
    if (revision === notificationRevision)
      closeNotificationWindow()
  }
  catch (error) {
    if (revision === notificationRevision) {
      actionPending.value = false
      actionError.value = t('feedback.failed', { error: formatError(error) })
      runBreakCountdown()
    }
  }
}

async function handleAction(action: string) {
  if (!visible.value || actionPending.value)
    return
  actionPending.value = true
  const revision = notificationRevision
  clearAutoDismiss()

  const shouldHoldForBreak = action === 'completed'
    && actionDurationSeconds.value > 0
    && actionEnabled.value

  if (props.preview) {
    actionPending.value = false
    if (shouldHoldForBreak)
      startBreakCountdown()
    else
      closeNotificationWindow()
    return
  }

  try {
    const progress = await invoke<{ remaining_seconds: number } | null>('respond_reminder', {
      notificationId: notificationId.value,
      reminderId: reminderId.value,
      action,
      holdNotification: shouldHoldForBreak,
    })

    if (revision !== notificationRevision)
      return
    actionPending.value = false
    if (shouldHoldForBreak) {
      startBreakCountdown(progress?.remaining_seconds ?? actionDurationSeconds.value)
      return
    }

    closeNotificationWindow()
  }
  catch (err) {
    if (revision === notificationRevision) {
      actionPending.value = false
      actionError.value = t('feedback.failed', { error: formatError(err) })
      startAutoDismiss()
    }
    console.error('Failed to respond:', err)
  }
}

async function handlePostpone(minutes: number) {
  if (!visible.value || actionPending.value)
    return
  if (props.preview) {
    closeNotificationWindow()
    return
  }
  actionPending.value = true
  const revision = notificationRevision
  clearAutoDismiss()

  try {
    await invoke('postpone_reminder', {
      notificationId: notificationId.value,
      reminderId: reminderId.value,
      minutes,
    })

    if (revision === notificationRevision)
      closeNotificationWindow()
  }
  catch (err) {
    if (revision === notificationRevision) {
      actionPending.value = false
      actionError.value = t('feedback.failed', { error: formatError(err) })
      startAutoDismiss()
    }
    console.error('Failed to postpone:', err)
  }
}
</script>

<template>
  <NotificationCard
    :style="previewRequest ? { height: `${windowSize.height}px` } : undefined"
    :content-height="contentHeight"
    :appearance="notificationAppearance"
    :collapsed="collapsed"
    :visible="visible"
    :pending="actionPending"
    :error="actionError"
    :visual="visual"
    :name="name"
    :message="message"
    :break-mode="breakMode"
    :action-title="actionTitle || t(reminderType === 'eye_care' ? 'notification.startEyeCare' : reminderType === 'rest' ? 'notification.startBreak' : 'ui.startAction')"
    :action-message="actionMessage || t('notification.breakMessage', { time: breakCountdownLabel })"
    :action-enabled="actionAvailable"
    :remaining-progress="actionDurationSeconds > 0 ? breakRemainingSeconds / actionDurationSeconds : 0"
    :remaining-seconds="breakRemainingSeconds"
    :countdown-running="breakCountdownRunning"
    :break-countdown-label="breakCountdownLabel"
    :pending-label="pendingLabel"
    :postpone-options="postponeOptions"
    :preview="preview"
    @complete="handleAction('completed')"
    @skip="handleAction('skipped')"
    @postpone="handlePostpone"
    @finish="closeBreakPrompt(true)"
    @collapse="collapsed = $event"
    @interaction="setInteracting"
  />
</template>
