<script setup lang="ts">
import type { CreateReminderRequest, Reminder, ReminderRuntime, RuntimeSnapshot, TimerTick } from './types/reminder'
import type { MascotStyle } from './utils/mascotStyles'
import type { NotificationPaletteSetting } from './utils/notificationPalette'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import { createReminder, deleteReminder, getReminderOverview, toggleReminder, updateReminder } from './api/reminder'
import AddReminderForm from './components/AddReminderForm.vue'
import AppIcon from './components/AppIcon.vue'
import AppResizeHandles from './components/AppResizeHandles.vue'
import AppTitleBar from './components/AppTitleBar.vue'
import ConfirmDialog from './components/ConfirmDialog.vue'
import ReminderCard from './components/ReminderCard.vue'
import SettingsPage from './components/SettingsPage.vue'
import StatsPage from './components/StatsPage.vue'
import { useAppEvents } from './composables/useAppEvents'
import { useTheme } from './composables/useTheme'
import { loadLanguage, useI18n } from './i18n'
import { getMascotAsset, mascotStyleOptions, normalizeMascotStyle } from './utils/mascotStyles'
import { applyPalette, normalizeNotificationPalette, resolveNotificationPalette } from './utils/notificationPalette'
import { withTimeout } from './utils/withTimeout'

const { applyTheme, themeRevision } = useTheme()

const mascotStyle = shallowRef<MascotStyle>('classic')
const notificationPalette = shallowRef<NotificationPaletteSetting>('auto')
const reminders = ref<Reminder[]>([])
const countdowns = ref<Record<string, number>>({})
const globallyPaused = ref(false)
const runtimeStates = ref<Record<string, ReminderRuntime>>({})
const suppression = ref<ReminderRuntime | null>(null)
let runtimeRevision = 0
let overviewRequest = 0
const showAddForm = ref(false)
const editingReminder = ref<Reminder | null>(null)
const globalTogglePending = ref(false)
const showSettings = ref(false)
const showStats = ref(false)
const pendingDeleteReminder = ref<Reminder | null>(null)
const deleteLoading = ref(false)
const addLoading = ref(false)
const addError = ref('')
const deleteError = ref('')
const actionError = ref('')
const loadError = ref('')
const loaded = ref(false)
const pendingToggles = ref(new Set<string>())
const { locale, t, formatError, clock } = useI18n()
const appEvents = useAppEvents(() => t('feedback.loadTimeout'))
const requests = new AbortController()
let eventsReady = false
let initializing = false
const activeMascotOption = computed(() => mascotStyleOptions.find(option => option.id === mascotStyle.value) ?? mascotStyleOptions[0])
const companionAvatar = computed(() => getMascotAsset(mascotStyle.value, activeMascotOption.value.previewState))
const emptyMascotAsset = computed(() => getMascotAsset(mascotStyle.value, 'snooze'))
const enabledCount = computed(() => reminders.value.filter(item => item.enabled).length)
const nextCountdownLabel = computed(() => {
  if (globallyPaused.value || suppression.value || !clock)
    return ''
  const upcoming = reminders.value
    .filter(item => item.enabled && typeof countdowns.value[item.id] === 'number')
    .map(item => countdowns.value[item.id]!)
    .filter(sec => Number.isFinite(sec) && sec > 0)
  if (upcoming.length === 0)
    return ''
  return t('ui.nextUp', { time: clock(Math.min(...upcoming)) })
})
const globalStatus = computed(() => {
  if (globallyPaused.value)
    return t(suppression.value?.reason === 'system' ? 'runtime.systemPaused' : 'reminder.paused')
  if (!suppression.value)
    return ''
  const until = suppression.value.until
  return until ? t('runtime.until', { time: new Date(`${until}Z`).toLocaleTimeString(locale.value, { hour: '2-digit', minute: '2-digit' }) }) : t('settings.dnd')
})
const companionSummary = computed(() => {
  if (globallyPaused.value)
    return t(suppression.value?.reason === 'system' ? 'runtime.systemPaused' : 'reminder.paused')
  if (suppression.value)
    return globalStatus.value
  if (enabledCount.value === 0)
    return t('app.keepRhythm')
  const countText = t('app.runningCount', { count: enabledCount.value })
  return nextCountdownLabel.value ? `${countText} · ${nextCountdownLabel.value}` : countText
})

function openCreate() {
  editingReminder.value = null
  addError.value = ''
  showAddForm.value = true
}
function openEdit(id: string) {
  editingReminder.value = reminders.value.find(reminder => reminder.id === id) || null
  addError.value = ''
  showAddForm.value = Boolean(editingReminder.value)
}
async function toggleAll() {
  if (globalTogglePending.value)
    return
  globalTogglePending.value = true
  actionError.value = ''
  try {
    await invoke('toggle_all_reminders', { enabled: globallyPaused.value })
    await loadReminders()
  }
  catch (error) {
    actionError.value = t('feedback.failed', { error: formatError(error) })
  }
  finally {
    globalTogglePending.value = false
  }
}

async function initialize() {
  if (initializing || requests.signal.aborted)
    return
  initializing = true
  loadError.value = ''
  overviewRequest++
  try {
    if (!eventsReady) {
      await appEvents.connect(listen => Promise.all([
        listen<TimerTick>('timer:tick', (event) => {
          if (globallyPaused.value || runtimeRevision > 0)
            return
          countdowns.value[event.payload.reminder_id] = Math.max(0, event.payload.remaining_seconds)
        }),
        listen('reminders:changed', () => { void loadReminders() }),
        listen<RuntimeSnapshot>('reminders:state', event => applyRuntime(event.payload)),
        listen('notification:queue-updated', () => { void loadReminders() }),
        listen('settings:changed', () => { void Promise.all([loadTheme(), loadLanguage()]) }),
        listen<string>('navigate', (event) => {
          if (event.payload === 'settings')
            showSettings.value = true
        }),
      ]))
      eventsReady = true
    }
    if (requests.signal.aborted)
      return
    void loadTheme()
    void loadLanguage()
    await loadReminders()
  }
  catch (error) {
    if (!requests.signal.aborted) {
      overviewRequest++
      loadError.value = t('feedback.failed', { error: formatError(error) })
    }
  }
  finally {
    initializing = false
  }
}

onMounted(initialize)

onUnmounted(() => {
  overviewRequest++
  requests.abort()
})

function applyRuntime(snapshot: RuntimeSnapshot) {
  if (snapshot.revision && snapshot.revision < runtimeRevision)
    return
  runtimeRevision = snapshot.revision || runtimeRevision
  globallyPaused.value = snapshot.all_paused
  countdowns.value = snapshot.countdowns
  runtimeStates.value = snapshot.states || {}
  suppression.value = snapshot.suppression || null
}

async function loadTheme() {
  const revision = themeRevision()
  try {
    const settings = await invoke<Record<string, string>>('get_all_settings')
    if (revision !== themeRevision())
      return
    mascotStyle.value = normalizeMascotStyle(settings.mascot_style)
    notificationPalette.value = normalizeNotificationPalette(settings.notification_palette)
    applyPalette(resolveNotificationPalette(notificationPalette.value, mascotStyle.value))
    if (settings.theme) {
      applyTheme(settings.theme)
      return
    }
    applyTheme('system')
    return
  }
  catch {
    // 忽略设置加载失败
  }

  const legacy = localStorage.getItem('app-settings')
  if (!legacy) {
    applyPalette(resolveNotificationPalette(notificationPalette.value, mascotStyle.value))
    return
  }

  try {
    const settings = JSON.parse(legacy)
    mascotStyle.value = normalizeMascotStyle(settings.mascotStyle)
    notificationPalette.value = normalizeNotificationPalette(settings.notificationPalette)
    applyPalette(resolveNotificationPalette(notificationPalette.value, mascotStyle.value))
    applyTheme(settings.theme || 'system')
  }
  catch {
    // 忽略旧设置解析失败
  }
}

async function loadReminders() {
  const request = ++overviewRequest
  try {
    const overview = await withTimeout(getReminderOverview(), t('feedback.loadTimeout'), requests.signal)
    if (request !== overviewRequest)
      return
    reminders.value = overview.reminders
    applyRuntime(overview)
    loadError.value = ''
    loaded.value = true
  }
  catch (err) {
    if (request === overviewRequest)
      loadError.value = t('feedback.failed', { error: formatError(err) })
  }
}

async function handleToggle(id: string) {
  if (pendingToggles.value.has(id))
    return
  pendingToggles.value.add(id)
  actionError.value = ''
  try {
    await toggleReminder(id)
    await loadReminders()
  }
  catch (err) {
    actionError.value = t('feedback.failed', { error: formatError(err) })
  }
  finally {
    pendingToggles.value.delete(id)
  }
}

async function handleDelete(id: string) {
  const target = reminders.value.find(reminder => reminder.id === id)
  if (!target)
    return

  pendingDeleteReminder.value = target
  deleteError.value = ''
}

async function handleDeleteConfirm() {
  const target = pendingDeleteReminder.value
  if (!target || deleteLoading.value)
    return

  try {
    deleteLoading.value = true
    deleteError.value = ''
    await deleteReminder(target.id)
    if (editingReminder.value?.id === target.id)
      showAddForm.value = false
    pendingDeleteReminder.value = null
    await loadReminders()
  }
  catch (err) {
    deleteError.value = t('feedback.failed', { error: formatError(err) })
  }
  finally {
    deleteLoading.value = false
  }
}

function handleDeleteCancel() {
  if (deleteLoading.value)
    return

  pendingDeleteReminder.value = null
}

async function handleAdd(data: CreateReminderRequest) {
  if (addLoading.value)
    return
  addLoading.value = true
  addError.value = ''
  try {
    if (editingReminder.value)
      await updateReminder(editingReminder.value.id, data)
    else
      await createReminder(data)
    showAddForm.value = false
    await loadReminders()
  }
  catch (err) {
    addError.value = t('feedback.failed', { error: formatError(err) })
  }
  finally {
    addLoading.value = false
  }
}
</script>

<template>
  <div class="app-shell">
    <AppTitleBar />
    <AppResizeHandles />

    <header class="app-header">
      <div class="title-block">
        <div class="title-copy">
          <h1 class="app-title">
            {{ t('ui.reminders') }}
          </h1>
        </div>
      </div>

      <div class="header-actions">
        <button class="header-button" :disabled="globalTogglePending || !loaded" type="button" :aria-label="t(globallyPaused ? 'ui.resumeAll' : 'ui.pauseAll')" :title="t(globallyPaused ? 'ui.resumeAll' : 'ui.pauseAll')" @click="toggleAll">
          <AppIcon :name="globallyPaused ? 'play' : 'pause'" />
        </button>
        <button class="header-button" type="button" :aria-label="t('app.stats')" :title="t('app.stats')" @click="showStats = true">
          <AppIcon name="stats" />
        </button>
        <button class="header-button" type="button" :aria-label="t('app.settings')" :title="t('app.settings')" @click="showSettings = true">
          <AppIcon name="settings" />
        </button>
        <button class="header-button header-button-primary" type="button" :aria-label="t('form.title')" :title="t('form.title')" @click="openCreate">
          <AppIcon name="plus" />
        </button>
      </div>
    </header>

    <main class="reminder-list">
      <button v-if="loaded && !loadError && reminders.length > 0" class="companion-strip" type="button" @click="showSettings = true">
        <span class="companion-avatar-wrap">
          <img :src="companionAvatar" alt="" class="companion-avatar" width="44" height="44">
        </span>
        <span class="companion-copy">
          <span class="companion-name">{{ t(activeMascotOption.nameKey) }}</span>
          <span class="companion-summary">{{ companionSummary }}</span>
        </span>
        <span class="companion-action">{{ t('ui.switchCompanion') }}</span>
      </button>
      <p v-if="globalStatus" class="global-status" role="status">
        {{ globalStatus }}
      </p>
      <p v-if="loadError" role="alert">
        {{ loadError }} <button type="button" @click="initialize">
          {{ t('feedback.retry') }}
        </button>
      </p>
      <p v-if="actionError" role="alert">
        {{ actionError }}
      </p>
      <p v-if="!loaded && !loadError" role="status">
        {{ t('feedback.loading') }}
      </p>
      <div v-if="loaded && !loadError && reminders.length === 0" class="empty-state">
        <img :src="emptyMascotAsset" :alt="t('app.emptyAlt')" class="empty-image">
        <h2 class="empty-title">
          {{ t('app.emptyTitle') }}
        </h2>
        <p class="empty-description">
          {{ t('app.emptyDescription') }}
        </p>
        <button class="empty-action" type="button" @click="openCreate">
          {{ t('app.addFirst') }}
        </button>
      </div>

      <TransitionGroup v-else name="list">
        <ReminderCard
          v-for="reminder in reminders"
          :key="reminder.id"
          :reminder="reminder"
          :remaining-seconds="countdowns[reminder.id]"
          :globally-paused="globallyPaused"
          :globally-suppressed="Boolean(suppression)"
          :runtime="runtimeStates[reminder.id]"
          :pending="pendingToggles.has(reminder.id)"
          @toggle="handleToggle"
          @edit="openEdit"
        />
      </TransitionGroup>
    </main>

    <Transition name="modal">
      <AddReminderForm
        v-if="showAddForm"
        :pending="addLoading"
        :error="addError"
        :reminder="editingReminder"
        @submit="handleAdd"
        @cancel="!addLoading && (showAddForm = false)"
        @delete="editingReminder && handleDelete(editingReminder.id)"
      />
    </Transition>

    <Transition name="modal">
      <SettingsPage
        v-if="showSettings"
        @close="showSettings = false"
      />
    </Transition>

    <Transition name="modal">
      <StatsPage
        v-if="showStats"
        :mascot-style="mascotStyle"
        @close="showStats = false"
      />
    </Transition>

    <Transition name="modal">
      <ConfirmDialog
        v-if="pendingDeleteReminder"
        :title="t('app.deleteConfirmTitle')"
        :message="t('app.deleteConfirmMessage', { name: pendingDeleteReminder.name })"
        :confirm-text="t('common.delete')"
        :cancel-text="t('common.cancel')"
        :loading="deleteLoading"
        :error="deleteError"
        danger
        @confirm="handleDeleteConfirm"
        @cancel="handleDeleteCancel"
      />
    </Transition>
  </div>
</template>

<style>
.app-shell {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
  background: var(--app-shell-bg);
  -webkit-backdrop-filter: blur(14px);
  backdrop-filter: blur(14px);
  transition: background 220ms ease;
}
.app-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 16px 10px; }
.title-block, .title-copy { min-width: 0; }
.app-title { margin: 0; font-size: 18px; font-weight: 700; color: var(--text-primary); line-height: 1.3; overflow-wrap: anywhere; }
.header-actions { display: flex; align-items: center; gap: 5px; flex-shrink: 0; }
.header-button { width: 34px; height: 34px; display: grid; place-items: center; border-radius: 10px; color: var(--text-secondary); transition: background-color 150ms ease, color 150ms ease, transform 120ms ease; }
.header-button:hover { background: var(--bg-secondary); color: var(--text-primary); }
.header-button:active { transform: scale(0.95); }
.header-button-primary { color: var(--color-primary-contrast); background: var(--color-primary); box-shadow: var(--shadow-sm); }
.header-button-primary:hover { background: var(--color-primary-hover); color: var(--color-primary-contrast); }
.reminder-list { flex: 1; overflow-y: auto; padding: 0 16px 16px; }
.companion-strip {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 11px;
  margin-bottom: 12px;
  padding: 9px 12px;
  border-radius: 16px;
  border: 1px solid var(--border-color);
  background: var(--card-surface);
  text-align: start;
  box-shadow: var(--shadow-sm);
  transition: border-color 150ms ease, box-shadow 150ms ease, transform 120ms ease;
}
.companion-strip:hover {
  border-color: var(--border-color-hover);
  box-shadow: var(--shadow-md);
}
.companion-strip:active { transform: scale(0.99); }
.companion-avatar-wrap {
  width: 46px;
  height: 46px;
  flex: 0 0 46px;
  border-radius: 13px;
  background: var(--bg-secondary);
  display: grid;
  place-items: center;
  overflow: hidden;
}
.companion-avatar { width: 42px; height: 42px; object-fit: contain; }
.companion-copy { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.companion-name { font-size: 13px; font-weight: 650; color: var(--text-primary); line-height: 1.3; }
.companion-summary { font-size: 11px; color: var(--text-secondary); line-height: 1.4; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.companion-action {
  flex-shrink: 0;
  padding: 5px 10px;
  border-radius: 999px;
  background: var(--color-primary-light);
  color: var(--color-primary);
  font-size: 11px;
  font-weight: 600;
}
.global-status { margin: 0 0 10px; padding: 9px 12px; border-radius: 11px; border: 1px solid var(--border-color); font-size: 12px; color: var(--text-secondary); background: var(--bg-secondary); }

.empty-state {
  min-height: calc(100vh - 120px);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 36px 20px 48px;
}

.empty-image {
  width: min(196px, 60%);
  max-width: 196px;
  margin-bottom: 18px;
}

.empty-title {
  margin: 0 0 8px;
  font-size: 22px;
  color: var(--text-primary);
}

.empty-description {
  margin: 0;
  max-width: 320px;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.7;
}

.empty-action {
  margin-top: 18px;
  padding: 10px 20px;
  border-radius: 999px;
  background: var(--color-primary);
  color: var(--color-primary-contrast);
  font-size: 13px;
  font-weight: 600;
  box-shadow: var(--shadow-md);
  transition: transform 0.18s ease, background-color 0.18s ease;
}

.empty-action:hover {
  transform: translateY(-1px);
  background: var(--color-primary-hover);
}

.empty-action:active {
  transform: scale(0.96);
}

.list-enter-active,
.list-leave-active {
  transition: opacity 0.25s, transform 0.35s cubic-bezier(0.16, 1, 0.3, 1);
}

.list-enter-from {
  opacity: 0;
  transform: translateY(18px);
}

.list-leave-to {
  opacity: 0;
  transform: translateY(-12px);
}

.list-move {
  transition: transform 0.35s cubic-bezier(0.16, 1, 0.3, 1);
}

.modal-enter-active {
  transition: opacity 0.25s ease;
}

.modal-enter-active :deep(.form-container),
.modal-enter-active :deep(.settings-container),
.modal-enter-active :deep(.stats-container),
.modal-enter-active :deep(.confirm-container) {
  animation: modal-in 0.28s cubic-bezier(0.16, 1, 0.3, 1);
}

.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-leave-active :deep(.form-container),
.modal-leave-active :deep(.settings-container),
.modal-leave-active :deep(.stats-container),
.modal-leave-active :deep(.confirm-container) {
  animation: modal-out 0.2s ease forwards;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

@keyframes modal-in {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes modal-out {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(0.96);
  }
}
</style>
