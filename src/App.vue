<script setup lang="ts">
import type { CreateReminderRequest, Reminder, TimerTick } from './types/reminder'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { createReminder, deleteReminder, getReminders, toggleReminder } from './api/reminder'
import AddReminderForm from './components/AddReminderForm.vue'
import AppResizeHandles from './components/AppResizeHandles.vue'
import AppTitleBar from './components/AppTitleBar.vue'
import ConfirmDialog from './components/ConfirmDialog.vue'
import ReminderCard from './components/ReminderCard.vue'
import SettingsPage from './components/SettingsPage.vue'
import StatsPage from './components/StatsPage.vue'
import { loadLanguage, useI18n } from './i18n'
import { emptyReminders } from './utils/reminderVisuals'

const reminders = ref<Reminder[]>([])
const countdowns = ref<Record<string, number>>({})
const showAddForm = ref(false)
const showSettings = ref(false)
const showStats = ref(false)
const pendingDeleteReminder = ref<Reminder | null>(null)
const deleteLoading = ref(false)
const { t } = useI18n()

const enabledCount = computed(() => reminders.value.filter(reminder => reminder.enabled).length)

// 监听定时器 tick 事件
let unlistenTick: (() => void) | null = null
let unlistenChanged: (() => void) | null = null
let unlistenNavigate: (() => void) | null = null

onMounted(async () => {
  await loadTheme()
  await loadLanguage()
  await loadReminders()

  unlistenTick = await listen<TimerTick>('timer:tick', (event) => {
    countdowns.value[event.payload.reminder_id] = Math.max(0, event.payload.remaining_seconds)
  })

  unlistenChanged = await listen('reminders:changed', () => {
    loadReminders()
  })

  unlistenNavigate = await listen<string>('navigate', (event) => {
    if (event.payload === 'settings') {
      showSettings.value = true
    }
  })
})

onUnmounted(() => {
  unlistenTick?.()
  unlistenChanged?.()
  unlistenNavigate?.()
})

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

async function loadTheme() {
  try {
    const settings = await invoke<Record<string, string>>('get_all_settings')
    if (settings.theme) {
      applyTheme(settings.theme)
      return
    }
  }
  catch {
    // 忽略设置加载失败
  }

  const legacy = localStorage.getItem('app-settings')
  if (!legacy)
    return

  try {
    const settings = JSON.parse(legacy)
    applyTheme(settings.theme || 'system')
  }
  catch {
    // 忽略旧设置解析失败
  }
}

async function loadReminders() {
  try {
    reminders.value = await getReminders()
    syncCountdownsFromReminders()
  }
  catch (err) {
    console.error('Failed to load reminders:', err)
  }
}

function syncCountdownsFromReminders() {
  const now = Date.now()
  const nextCountdowns: Record<string, number> = {}

  for (const reminder of reminders.value) {
    if (!reminder.enabled || !reminder.next_trigger)
      continue

    const triggerTime = new Date(`${reminder.next_trigger}Z`).getTime()
    if (Number.isNaN(triggerTime))
      continue

    nextCountdowns[reminder.id] = Math.max(0, Math.floor((triggerTime - now) / 1000))
  }

  countdowns.value = nextCountdowns
}

async function handleToggle(id: string) {
  try {
    await toggleReminder(id)
    await loadReminders()
  }
  catch (err) {
    console.error('Failed to toggle reminder:', err)
  }
}

async function handleDelete(id: string) {
  const target = reminders.value.find(reminder => reminder.id === id)
  if (!target)
    return

  pendingDeleteReminder.value = target
}

async function handleDeleteConfirm() {
  const target = pendingDeleteReminder.value
  if (!target)
    return

  try {
    deleteLoading.value = true
    await deleteReminder(target.id)
    pendingDeleteReminder.value = null
    await loadReminders()
  }
  catch (err) {
    console.error('Failed to delete reminder:', err)
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
  try {
    await createReminder(data)
    showAddForm.value = false
    await loadReminders()
  }
  catch (err) {
    console.error('Failed to create reminder:', err)
  }
}
</script>

<template>
  <div class="app-shell card-base border-none rounded-none h-screen flex flex-col overflow-hidden">
    <AppTitleBar />
    <AppResizeHandles />

    <header class="app-header">
      <div class="title-block">
        <div class="status-indicator" />
        <div class="title-copy">
          <h1 class="app-title">
            {{ t('app.runningCount', { count: enabledCount }) }}
          </h1>
          <p class="app-subtitle">
            {{ t('app.keepRhythm') }}
          </p>
        </div>
      </div>

      <div class="header-actions">
        <button class="header-button header-button-text" type="button" @click="showStats = true">
          {{ t('app.stats') }}
        </button>
        <button class="header-button header-button-text" type="button" @click="showSettings = true">
          {{ t('app.settings') }}
        </button>
        <button class="header-button header-button-primary" type="button" @click="showAddForm = true">
          <span class="header-button-plus">+</span>
        </button>
      </div>
    </header>

    <main class="reminder-list">
      <div v-if="reminders.length === 0" class="empty-state">
        <img :src="emptyReminders" :alt="t('app.emptyAlt')" class="empty-image">
        <h2 class="empty-title">
          {{ t('app.emptyTitle') }}
        </h2>
        <p class="empty-description">
          {{ t('app.emptyDescription') }}
        </p>
        <button class="empty-action" type="button" @click="showAddForm = true">
          {{ t('app.addFirst') }}
        </button>
      </div>

      <TransitionGroup v-else name="list">
        <ReminderCard
          v-for="reminder in reminders"
          :key="reminder.id"
          :reminder="reminder"
          :remaining-seconds="countdowns[reminder.id]"
          @toggle="handleToggle"
          @delete="handleDelete"
        />
      </TransitionGroup>
    </main>

    <Transition name="modal">
      <AddReminderForm
        v-if="showAddForm"
        @submit="handleAdd"
        @cancel="showAddForm = false"
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
  background:
    radial-gradient(circle at top left, rgba(47, 159, 216, 0.12), transparent 34%),
    radial-gradient(circle at top right, rgba(246, 179, 91, 0.1), transparent 28%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.88), rgba(255, 255, 255, 0.74));
}

[data-theme='dark'] .app-shell {
  background:
    radial-gradient(circle at top left, rgba(47, 159, 216, 0.2), transparent 34%),
    radial-gradient(circle at top right, rgba(246, 179, 91, 0.16), transparent 28%),
    linear-gradient(180deg, rgba(20, 24, 31, 0.96), rgba(16, 20, 28, 0.92));
}

.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 14px 20px 16px;
}

.title-block {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.status-indicator {
  position: relative;
  width: 12px;
  height: 32px;
  flex-shrink: 0;
  border-radius: 999px;
  background:
    linear-gradient(180deg, rgba(47, 159, 216, 0.96), rgba(246, 179, 91, 0.82));
  box-shadow:
    0 10px 20px rgba(47, 159, 216, 0.18),
    inset 0 1px 0 rgba(255, 255, 255, 0.52);
}

.status-indicator::after {
  position: absolute;
  top: 5px;
  left: 3px;
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.92);
  content: '';
}

[data-theme='dark'] .status-indicator {
  background: linear-gradient(180deg, rgba(47, 159, 216, 0.86), rgba(246, 179, 91, 0.62));
  box-shadow:
    0 10px 20px rgba(2, 6, 23, 0.26),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
}

.title-copy {
  min-width: 0;
}

.app-title {
  margin: 0;
  min-width: 0;
  overflow: hidden;
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-subtitle {
  margin: 3px 0 0;
  overflow: hidden;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.header-button {
  width: 38px;
  height: 38px;
  border-radius: 14px;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background: rgba(255, 255, 255, 0.72);
  color: var(--text-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  transition:
    transform 0.2s ease,
    background-color 0.2s ease,
    border-color 0.2s ease,
    box-shadow 0.2s ease;
}

.header-button:hover {
  transform: translateY(-1px);
  border-color: rgba(79, 140, 255, 0.26);
  box-shadow: 0 12px 24px rgba(15, 23, 42, 0.08);
}

.header-button:active {
  transform: scale(0.95);
}

[data-theme='dark'] .header-button {
  background: rgba(24, 28, 38, 0.88);
  border-color: rgba(148, 163, 184, 0.16);
}

.header-button-primary {
  background: linear-gradient(135deg, rgba(47, 159, 216, 0.92), rgba(79, 140, 255, 0.94));
  color: white;
  border-color: transparent;
}

.header-button-text {
  width: auto;
  min-width: 44px;
  padding: 0 12px;
  font-size: 13px;
  font-weight: 600;
}

.header-button-plus {
  font-size: 22px;
  line-height: 1;
}

.reminder-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 20px 18px;
}

.reminder-list::-webkit-scrollbar {
  width: 4px;
}

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
  width: min(240px, 70%);
  max-width: 240px;
  margin-bottom: 18px;
}

.empty-title {
  margin: 0 0 8px;
  font-size: 24px;
  color: var(--text-primary);
}

.empty-description {
  margin: 0;
  max-width: 320px;
  font-size: 14px;
  color: var(--text-secondary);
  line-height: 1.7;
}

.empty-action {
  margin-top: 18px;
  padding: 11px 18px;
  border-radius: 999px;
  background: linear-gradient(135deg, #2f9fd8, #4f8cff);
  color: white;
  font-size: 14px;
  font-weight: 600;
  box-shadow: 0 12px 22px rgba(47, 159, 216, 0.22);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.empty-action:hover {
  transform: translateY(-1px);
  box-shadow: 0 18px 28px rgba(47, 159, 216, 0.26);
}

.empty-action:active {
  transform: scale(0.96);
}

.list-enter-active,
.list-leave-active {
  transition: all 0.35s cubic-bezier(0.16, 1, 0.3, 1);
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
