<script setup lang="ts">
import type { NotificationPreviewRequest } from './types/notificationPreview'
import type { MascotStyle } from './utils/mascotStyles'
import type { NotificationSoundPreset } from './utils/notificationSound'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import NotificationCard from './components/NotificationCard.vue'
import { loadLanguage, useI18n } from './i18n'
import { normalizeMascotStyle } from './utils/mascotStyles'
import { playNotificationSound } from './utils/notificationSound'
import { getLocalizedReminderVisual } from './utils/reminderVisuals'

const props = defineProps<{
  preview?: boolean
  previewRequest?: NotificationPreviewRequest
}>()
const emit = defineEmits<{ closed: [] }>()
const mascotStyle = shallowRef<MascotStyle>('classic')
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
const breakRemainingSeconds = ref(0)
const pendingCount = ref(0)
const { locale, t } = useI18n()

interface SettingsPayload {
  theme?: string
  mascotStyle?: MascotStyle
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

const breakCountdownLabel = computed(() => {
  const safe = Math.max(0, breakRemainingSeconds.value)
  const minutes = Math.floor(safe / 60)
  const seconds = safe % 60
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
})

const pendingLabel = computed(() => {
  const count = Math.max(0, pendingCount.value)
  return count > 0 ? t('notification.pendingCount', { count }) : ''
})

function buildPostponeOptions(values?: number[]): NotificationOption[] {
  const source = values && values.length > 0 ? values : [5, 10, 15]
  return source.map(minutes => ({
    label: `${minutes} ${t('common.minutes')}`,
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

function applyTheme(theme: string) {
  const root = document.documentElement
  if (theme === 'dark') {
    root.setAttribute('data-theme', 'dark')
    return
  }

  if (theme === 'light') {
    root.removeAttribute('data-theme')
    return
  }

  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  if (prefersDark) {
    root.setAttribute('data-theme', 'dark')
  }
  else {
    root.removeAttribute('data-theme')
  }
}

async function loadDisplaySettings() {
  try {
    const settings = await invoke<Record<string, string>>('get_all_settings')
    mascotStyle.value = normalizeMascotStyle(settings.mascot_style)
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
let unlistenSystemResumed: (() => void) | null = null
let systemTimersPaused = false
let latestQueueRevision = -1
let notificationRevision = 0
let actionPending = false
let disposed = false

async function showPreview(request: NotificationPreviewRequest) {
  const revision = ++notificationRevision
  clearAutoDismiss()
  stopBreakCountdown()
  visible.value = false
  await loadLanguage({ language: request.settings.language || locale.value })
  if (disposed || revision !== notificationRevision)
    return

  mascotStyle.value = normalizeMascotStyle(request.mascotStyle)
  reminderType.value = request.reminderType
  reminderIcon.value = request.reminderType
  applyTheme(request.settings.theme)
  notificationDuration.value = Math.min(Math.max(request.settings.notificationDuration || 30, 5), 120) * 1000
  postponeOptions.value = buildPostponeOptions(request.settings.postponeOptions)
  name.value = visual.value.defaultName
  message.value = visual.value.shortMessage
  actionEnabled.value = request.reminderType !== 'drink'
  actionDurationSeconds.value = visual.value.defaultActionDurationSeconds || 0
  actionTitle.value = t('notification.startBreak')
  actionMessage.value = ''
  actionPending = false
  breakMode.value = false
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

  unlistenShow = await appWindow.listen<NotificationPayload>('notification:show', (event) => {
    const data = event.payload
    if (!data?.notification_id || !data?.reminder_id || !data?.name) {
      return
    }

    if (data.queue_revision < latestQueueRevision)
      return
    latestQueueRevision = data.queue_revision

    notificationRevision += 1
    actionPending = false
    clearAutoDismiss()
    stopBreakCountdown()
    breakMode.value = false
    name.value = data.name
    message.value = data.message || ''
    reminderId.value = data.reminder_id
    notificationId.value = data.notification_id
    reminderType.value = data.reminder_type || 'custom'
    reminderIcon.value = data.icon || data.reminder_type || 'custom_star'
    actionEnabled.value = Boolean(data.action_enabled)
    actionTitle.value = data.action_title || t('notification.startBreak')
    actionMessage.value = data.action_message || t('notification.breakMessage', { time: breakCountdownLabel.value })
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

  if (systemTimersPaused) {
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
  if (autoDismissRemainingMs === null || !visible.value || breakMode.value)
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
  if (breakTimer) {
    clearInterval(breakTimer)
    breakTimer = null
  }
}

function runBreakCountdown() {
  if (systemTimersPaused)
    return

  breakTimer = setInterval(() => {
    breakRemainingSeconds.value = Math.max(0, breakRemainingSeconds.value - 1)
    if (breakRemainingSeconds.value <= 0) {
      stopBreakCountdown()
      void closeBreakPrompt(false)
    }
  }, 1000)
}

function startBreakCountdown() {
  stopBreakCountdown()
  breakMode.value = true
  breakRemainingSeconds.value = actionDurationSeconds.value
  runBreakCountdown()
}

function pauseLocalTimers() {
  if (systemTimersPaused)
    return

  systemTimersPaused = true
  pauseAutoDismiss()
  stopBreakCountdown()
}

function resumeLocalTimers() {
  if (!systemTimersPaused)
    return

  systemTimersPaused = false
  if (breakMode.value) {
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
  actionPending = false
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
  if (!visible.value || actionPending)
    return
  if (props.preview) {
    closeNotificationWindow()
    return
  }
  actionPending = true
  const revision = notificationRevision
  stopBreakCountdown()
  breakMode.value = false

  try {
    await invoke('release_notification', {
      notificationId: notificationId.value,
      reminderId: reminderId.value,
      finishBreakNow,
    })
  }
  catch (err) {
    console.error('Failed to release notification:', err)
  }
  finally {
    if (revision === notificationRevision)
      closeNotificationWindow()
  }
}

async function handleAction(action: string) {
  if (!visible.value || actionPending)
    return
  actionPending = true
  const revision = notificationRevision
  clearAutoDismiss()

  const shouldHoldForBreak = action === 'completed'
    && actionDurationSeconds.value > 0
    && actionEnabled.value

  if (props.preview) {
    actionPending = false
    if (shouldHoldForBreak)
      startBreakCountdown()
    else
      closeNotificationWindow()
    return
  }

  try {
    await invoke('respond_reminder', {
      notificationId: notificationId.value,
      reminderId: reminderId.value,
      action,
      holdNotification: shouldHoldForBreak,
    })

    if (revision !== notificationRevision)
      return
    actionPending = false
    if (shouldHoldForBreak) {
      startBreakCountdown()
      return
    }

    closeNotificationWindow()
  }
  catch (err) {
    if (revision === notificationRevision) {
      actionPending = false
      startAutoDismiss()
    }
    console.error('Failed to respond:', err)
  }
}

async function handlePostpone(minutes: number) {
  if (!visible.value || actionPending)
    return
  if (props.preview) {
    closeNotificationWindow()
    return
  }
  actionPending = true
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
      actionPending = false
      startAutoDismiss()
    }
    console.error('Failed to postpone:', err)
  }
}
</script>

<template>
  <NotificationCard
    :visible="visible"
    :visual="visual"
    :name="name"
    :message="message"
    :break-mode="breakMode"
    :action-title="actionTitle"
    :action-message="actionMessage"
    :break-countdown-label="breakCountdownLabel"
    :pending-label="pendingLabel"
    :postpone-options="postponeOptions"
    :preview="preview"
    @complete="handleAction('completed')"
    @skip="handleAction('skipped')"
    @postpone="handlePostpone"
    @finish="closeBreakPrompt(true)"
  />
</template>
