<script setup lang="ts">
import type { ReminderVisual } from '../utils/reminderVisuals'
import { computed } from 'vue'
import { useI18n } from '../i18n'

const props = defineProps<{
  visible: boolean
  visual: ReminderVisual
  name: string
  message: string
  breakMode: boolean
  actionTitle: string
  actionMessage: string
  breakCountdownLabel: string
  pendingLabel: string
  postponeOptions: { label: string, minutes: number }[]
  preview?: boolean
}>()
const emit = defineEmits<{
  complete: []
  skip: []
  postpone: [minutes: number]
  finish: []
}>()
const { t } = useI18n()
const notificationStyle = computed(() => ({
  '--notification-accent': props.visual.accent,
  '--notification-soft': props.visual.accentSoft,
  '--notification-border': props.visual.borderSoft,
}))
</script>

<template>
  <div class="notification-wrapper" :class="{ 'notification-wrapper-visible': visible }">
    <div v-if="visible" class="notification-shell" :style="notificationStyle" :aria-label="preview ? t('settings.previewReminder') : undefined">
      <div class="notification-visual">
        <img
          v-if="visual.mascotAsset"
          :src="breakMode ? visual.statusAsset || visual.mascotAsset : visual.mascotAsset"
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
            {{ preview ? t('settings.previewBadge') : visual.label }}
          </span>
          <p v-if="pendingLabel" class="notification-queue">
            {{ pendingLabel }}
          </p>
          <h2 class="notification-title" :title="actionTitle">
            {{ actionTitle }}
          </h2>
          <p class="notification-message">
            {{ actionMessage || t('notification.breakMessage', { time: breakCountdownLabel }) }}
          </p>

          <div class="break-countdown">
            {{ breakCountdownLabel }}
          </div>

          <button class="complete-button" type="button" @click="emit('finish')">
            {{ t('notification.finishBreak') }}
          </button>
        </template>

        <template v-else>
          <span class="notification-tag">
            {{ preview ? t('settings.previewBadge') : visual.label }}
          </span>
          <p v-if="pendingLabel" class="notification-queue">
            {{ pendingLabel }}
          </p>
          <h2 class="notification-title" :title="name">
            {{ name }}
          </h2>
          <p class="notification-message" :title="message">
            {{ message }}
          </p>

          <div class="action-grid">
            <button class="complete-button" type="button" @click="emit('complete')">
              {{ t('notification.complete') }}
            </button>

            <button class="skip-button" type="button" @click="emit('skip')">
              {{ t('notification.skip') }}
            </button>
          </div>

          <div class="postpone-grid">
            <button
              v-for="option in postponeOptions"
              :key="option.minutes"
              class="postpone-button"
              type="button"
              @click="emit('postpone', option.minutes)"
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
  padding: 16px;
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
    0 4px 10px rgba(15, 23, 42, 0.12),
    0 1px 3px rgba(15, 23, 42, 0.08);
}

[data-theme='dark'] .notification-shell {
  background:
    radial-gradient(circle at top left, rgba(255, 255, 255, 0.06), transparent 45%),
    linear-gradient(180deg, rgba(24, 28, 37, 0.96), rgba(17, 21, 30, 0.94));
  box-shadow:
    0 4px 12px rgba(2, 6, 23, 0.32),
    0 1px 3px rgba(2, 6, 23, 0.24);
}

.notification-visual {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: 18px;
  border: 1px solid var(--notification-border);
  background: linear-gradient(180deg, var(--bg-secondary), var(--notification-soft));
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
  min-height: 0;
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
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  flex-shrink: 0;
}

.notification-message {
  margin: 4px 0 0;
  font-size: 13px;
  line-height: 1.35;
  color: var(--text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.notification-queue {
  margin: 6px 0 0;
  font-size: 11px;
  line-height: 1.4;
  color: var(--text-secondary);
}

.complete-button {
  padding: 8px 12px;
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
  margin-top: 8px;
  flex-shrink: 0;
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
  margin-top: 6px;
  flex-shrink: 0;
}

.postpone-button {
  padding: 7px 0;
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
  padding: 8px 12px;
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

@media (prefers-reduced-motion: reduce) {
  .notification-wrapper-visible { animation: none; }
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
