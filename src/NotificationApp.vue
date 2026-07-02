<script setup lang="ts">
import type { NotificationSoundPreset } from './utils/notificationSound'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { loadLanguage, useI18n } from './i18n'
import { playNotificationSound } from './utils/notificationSound'
import { getLocalizedReminderVisual } from './utils/reminderVisuals'

const name = ref('')
const message = ref('')
const reminderId = ref('')
const reminderType = ref('custom')
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
  reminder_id?: string
  name?: string
  message?: string
  reminder_type?: string
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
  current_reminder_id?: string | null
  pending_count?: number
}

const visual = computed(() => getLocalizedReminderVisual(reminderType.value, locale.value))
const notificationStyle = computed(() => ({
  '--notification-accent': visual.value.accent,
  '--notification-soft': visual.value.accentSoft,
  '--notification-border': visual.value.borderSoft,
}))

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

onMounted(async () => {
  const appWindow = getCurrentWebviewWindow()
  await loadDisplaySettings()

  unlistenShow = await appWindow.listen<NotificationPayload>('notification:show', (event) => {
    const data = event.payload
    if (!data?.reminder_id || !data?.name) {
      return
    }

    stopBreakCountdown()
    breakMode.value = false
    name.value = data.name
    message.value = data.message || ''
    reminderId.value = data.reminder_id
    reminderType.value = data.reminder_type || 'custom'
    actionEnabled.value = Boolean(data.action_enabled)
    actionTitle.value = data.action_title || t('notification.startBreak')
    actionMessage.value = data.action_message || t('notification.breakMessage', { time: breakCountdownLabel.value })
    actionDurationSeconds.value = data.action_duration_seconds || 0
    pendingCount.value = Math.max(0, Number(data.pending_count || 0))
    visible.value = true

    void loadDisplaySettings().then(() => {
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
    if (!data)
      return

    if (data.current_reminder_id && data.current_reminder_id !== reminderId.value)
      return

    if (!data.current_reminder_id && !visible.value)
      return

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

async function closeNotificationWindow() {
  visible.value = false
  breakMode.value = false
  clearAutoDismiss()
  stopBreakCountdown()
  pendingCount.value = 0
  const appWindow = getCurrentWebviewWindow()
  await appWindow.hide()
}

async function closeBreakPrompt(finishBreakNow: boolean) {
  stopBreakCountdown()
  breakMode.value = false

  try {
    await invoke('release_notification', {
      reminderId: reminderId.value,
      finishBreakNow,
    })
  }
  catch (err) {
    console.error('Failed to release notification:', err)
  }
  finally {
    await closeNotificationWindow()
  }
}

async function handleAction(action: string) {
  clearAutoDismiss()

  const shouldHoldForBreak = action === 'completed'
    && actionDurationSeconds.value > 0
    && actionEnabled.value

  try {
    await invoke('respond_reminder', {
      reminderId: reminderId.value,
      action,
      holdNotification: shouldHoldForBreak,
    })

    if (shouldHoldForBreak) {
      startBreakCountdown()
      return
    }

    await closeNotificationWindow()
  }
  catch (err) {
    console.error('Failed to respond:', err)
  }
}

async function handlePostpone(minutes: number) {
  clearAutoDismiss()

  try {
    await invoke('postpone_reminder', {
      reminderId: reminderId.value,
      minutes,
    })

    await closeNotificationWindow()
  }
  catch (err) {
    console.error('Failed to postpone:', err)
  }
}
</script>

<template>
  <div class="notification-wrapper" :class="{ 'notification-wrapper-visible': visible }">
    <div class="notification-shell" :style="notificationStyle">
      <div class="notification-visual">
        <img
          v-if="visual.mascotAsset"
          :src="visual.mascotAsset"
          :alt="visual.label"
          class="notification-image"
        >
        <img
          v-else-if="visual.iconAsset"
          :src="visual.iconAsset"
          :alt="visual.label"
          class="notification-image"
        >
        <span v-else class="notification-fallback-icon">{{ visual.iconText }}</span>
      </div>

      <div class="notification-content">
        <template v-if="breakMode">
          <span class="notification-tag">
            {{ visual.label }}
          </span>
          <p v-if="pendingLabel" class="notification-queue">
            {{ pendingLabel }}
          </p>
          <h2 class="notification-title">
            {{ actionTitle }}
          </h2>
          <p class="notification-message">
            {{ actionMessage || t('notification.breakMessage', { time: breakCountdownLabel }) }}
          </p>

          <div class="break-countdown">
            {{ breakCountdownLabel }}
          </div>

          <button class="complete-button" type="button" @click="closeBreakPrompt(true)">
            {{ t('notification.finishBreak') }}
          </button>
        </template>

        <template v-else>
          <span class="notification-tag">
            {{ visual.label }}
          </span>
          <p v-if="pendingLabel" class="notification-queue">
            {{ pendingLabel }}
          </p>
          <h2 class="notification-title">
            {{ name }}
          </h2>
          <p class="notification-message">
            {{ message }}
          </p>

          <div class="action-grid">
            <button class="complete-button" type="button" @click="handleAction('completed')">
              {{ t('notification.complete') }}
            </button>

            <button class="skip-button" type="button" @click="handleAction('skipped')">
              {{ t('notification.skip') }}
            </button>
          </div>

          <div class="postpone-grid">
            <button
              v-for="option in postponeOptions"
              :key="option.minutes"
              class="postpone-button"
              type="button"
              @click="handlePostpone(option.minutes)"
            >
              {{ option.label }}
            </button>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.notification-wrapper {
  width: 100%;
  height: 100%;
  display: flex;
  padding: 12px;
  background: transparent;
  overflow: hidden;
}

.notification-wrapper-visible {
  animation: slide-in 0.35s cubic-bezier(0.16, 1, 0.3, 1);
}

.notification-shell {
  width: 100%;
  height: 100%;
  min-height: 0;
  display: grid;
  grid-template-columns: 104px minmax(0, 1fr);
  gap: 12px;
  padding: 12px;
  overflow: hidden;
  border-radius: 24px;
  border: 1px solid var(--notification-border);
  background:
    radial-gradient(circle at top left, rgba(255, 255, 255, 0.48), transparent 45%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(255, 255, 255, 0.9));
  box-shadow:
    0 22px 42px rgba(15, 23, 42, 0.16),
    0 10px 18px rgba(15, 23, 42, 0.08);
}

[data-theme='dark'] .notification-shell {
  background:
    radial-gradient(circle at top left, rgba(255, 255, 255, 0.06), transparent 45%),
    linear-gradient(180deg, rgba(24, 28, 37, 0.96), rgba(17, 21, 30, 0.94));
  box-shadow:
    0 24px 50px rgba(2, 6, 23, 0.42),
    0 10px 20px rgba(2, 6, 23, 0.3);
}

.notification-visual {
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: 18px;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.54), var(--notification-soft));
}

.notification-image {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.notification-fallback-icon {
  font-size: 28px;
  font-weight: 700;
  color: var(--notification-accent);
}

.notification-content {
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.notification-tag {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--notification-accent);
}

.notification-title {
  margin: 4px 0 0;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1.25;
}

.notification-message {
  margin: 6px 0 0;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.notification-queue {
  margin: 6px 0 0;
  font-size: 11px;
  line-height: 1.4;
  color: var(--text-secondary);
}

.complete-button {
  padding: 9px 12px;
  border-radius: 14px;
  background: linear-gradient(135deg, var(--notification-accent), rgba(79, 140, 255, 0.94));
  color: white;
  font-size: 13px;
  font-weight: 700;
  box-shadow: 0 12px 20px rgba(15, 23, 42, 0.14);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.complete-button:hover {
  box-shadow: 0 16px 24px rgba(15, 23, 42, 0.18);
}

.action-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
  margin-top: 12px;
}

.complete-button:active,
.postpone-button:active,
.skip-button:active {
  transform: scale(0.97);
}

.postpone-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  margin-top: 8px;
}

.postpone-button {
  padding: 8px 0;
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background: rgba(255, 255, 255, 0.7);
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
  transition:
    background-color 0.2s ease,
    border-color 0.2s ease,
    transform 0.2s ease;
}

[data-theme='dark'] .postpone-button {
  background: rgba(30, 35, 46, 0.84);
}

.postpone-button:hover {
  border-color: var(--notification-border);
  background: var(--notification-soft);
}

.skip-button {
  padding: 9px 12px;
  border-radius: 14px;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background: rgba(255, 255, 255, 0.72);
  color: var(--text-primary);
  font-size: 13px;
  font-weight: 700;
  transition:
    background-color 0.2s ease,
    border-color 0.2s ease,
    transform 0.2s ease;
}

[data-theme='dark'] .skip-button {
  background: rgba(30, 35, 46, 0.84);
}

.skip-button:hover {
  border-color: var(--notification-border);
  background: var(--notification-soft);
}

.break-countdown {
  margin-top: 10px;
  font-size: 28px;
  font-weight: 700;
  color: var(--notification-accent);
  letter-spacing: -0.04em;
  font-variant-numeric: tabular-nums;
}

@keyframes slide-in {
  from {
    transform: translateX(24px);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}
</style>
