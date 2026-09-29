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
  actionEnabled?: boolean
  remainingProgress?: number
  preview?: boolean
}>()
const emit = defineEmits<{
  complete: []
  skip: []
  postpone: [minutes: number]
  finish: []
}>()
const { t } = useI18n()
const primaryLabel = computed(() => {
  if (props.actionEnabled) {
    if (props.visual.type === 'eye_care')
      return t('notification.startEyeCare')
    if (props.visual.type === 'rest')
      return t('notification.startBreak')
    return props.actionTitle || t('notification.startBreak')
  }
  return t(props.visual.type === 'drink' ? 'notification.drank' : 'notification.complete')
})
const countdownMessage = computed(() => {
  if (props.actionMessage)
    return props.actionMessage
  if (props.visual.type === 'eye_care')
    return t('notification.eyeHint')
  return props.visual.type === 'rest' ? t('notification.restHint') : props.message
})
const imageAsset = computed(() => {
  if (props.breakMode && props.visual.type !== 'eye_care')
    return props.visual.statusAsset || props.visual.mascotAsset || props.visual.iconAsset
  return props.visual.mascotAsset || props.visual.iconAsset
})
const progress = computed(() => Math.max(0, Math.min(1, props.remainingProgress ?? 1)))
</script>

<template>
  <div class="notification-wrapper" :class="{ 'notification-wrapper-visible': visible }">
    <section
      v-if="visible"
      class="notification-shell"
      :class="[`notification-${visual.type}`, { 'notification-counting': breakMode }]"
      :aria-label="preview ? t('settings.previewReminder') : visual.label"
    >
      <header class="notification-header">
        <span class="notification-tag">{{ preview ? t('settings.previewBadge') : visual.label }}</span>
        <span v-if="pendingLabel" class="notification-queue" :title="pendingLabel">{{ pendingLabel }}</span>
        <button v-if="!breakMode" class="skip-button" type="button" @click="emit('skip')">
          {{ t('notification.skip') }}
        </button>
      </header>

      <div class="notification-body">
        <div class="notification-visual" :class="{ 'notification-visual-mascot': visual.mascotAsset }">
          <img v-if="imageAsset" :src="imageAsset" alt="" class="notification-image" width="88" height="88">
          <span v-else class="notification-fallback-icon" aria-hidden="true">{{ visual.iconText }}</span>
        </div>
        <div class="notification-content">
          <h2 class="notification-title" :title="breakMode ? actionTitle : name">
            {{ breakMode ? actionTitle : name }}
          </h2>
          <p class="notification-message" :title="breakMode ? countdownMessage : message">
            {{ breakMode ? countdownMessage : message }}
          </p>
          <div v-if="breakMode" class="countdown-row">
            <span class="break-countdown" role="timer" :aria-label="t('notification.remaining')">{{ breakCountdownLabel }}</span>
            <span class="countdown-label">{{ t('notification.remaining') }}</span>
          </div>
          <button v-else class="complete-button" type="button" :title="primaryLabel" @click="emit('complete')">
            {{ primaryLabel }}
          </button>
        </div>
      </div>

      <footer v-if="breakMode" class="countdown-footer">
        <div class="countdown-track" aria-hidden="true">
          <div class="countdown-fill" :style="{ transform: `scaleX(${progress})` }" />
        </div>
        <button class="complete-button finish-button" type="button" @click="emit('finish')">
          {{ t(visual.type === 'rest' ? 'notification.finishBreak' : 'notification.complete') }}
        </button>
      </footer>
      <footer v-else-if="postponeOptions.length" class="postpone-grid" :aria-label="t('notification.later')">
        <span class="postpone-label">{{ t('notification.later') }}</span>
        <div class="postpone-options">
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
      </footer>
    </section>
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
  animation: slide-in 240ms ease-out;
}

.notification-shell {
  --action-bg: #e9edf1;
  --action-hover: #dde3e9;
  --action-text: #34485d;
  --bg-primary: #ffffff;
  --bg-secondary: #f5f5f7;
  --text-primary: #1d1d1f;
  --text-secondary: #636366;
  --border-color: #e5e5e8;
  --font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI Variable Text', 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', Roboto, 'Helvetica Neue', Arial, sans-serif;
  --notification-accent: #83838c;
  --notification-ink: #83838c;
  --notification-soft: #f0f3f7;
  width: 100%;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 14px;
  overflow: hidden;
  border-radius: 20px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  box-shadow: 0 2px 6px rgb(0 0 0 / 5%), 0 8px 18px rgb(0 0 0 / 6%);
  font-family: var(--font-family);
}

.notification-drink {
  --action-bg: #e1edf7;
  --action-hover: #d3e3f1;
  --action-text: #385d7c;
  --notification-accent: #7298b7;
  --notification-ink: #385d7c;
  --notification-soft: #eef4f9;
}

.notification-rest {
  --action-bg: #e5eddf;
  --action-hover: #d7e3ce;
  --action-text: #4c6643;
  --notification-accent: #8aa07b;
  --notification-ink: #4c6643;
  --notification-soft: #f1f5ee;
}

.notification-eye_care {
  --action-bg: #ddedeb;
  --action-hover: #cce2df;
  --action-text: #376762;
  --notification-accent: #78a6a0;
  --notification-ink: #376762;
  --notification-soft: #edf5f4;
}

[data-theme='dark'] .notification-shell {
  --action-bg: #3a4048;
  --action-hover: #454c56;
  --action-text: #e1e6ed;
  --bg-primary: #242426;
  --bg-secondary: #2c2c2e;
  --text-primary: #f5f5f7;
  --text-secondary: #b1b1b8;
  --border-color: #3a3a3c;
  --notification-ink: #b5b5bf;
  --notification-soft: #252e39;
  box-shadow: 0 4px 12px rgb(2 6 23 / 32%);
}

[data-theme='dark'] .notification-drink {
  --action-bg: #2c4052;
  --action-hover: #364e63;
  --action-text: #bed6eb;
  --notification-accent: #8fb1ce;
  --notification-ink: #bed6eb;
  --notification-soft: #283440;
}

[data-theme='dark'] .notification-rest {
  --action-bg: #354332;
  --action-hover: #42533d;
  --action-text: #ccdcbc;
  --notification-accent: #a6bb92;
  --notification-ink: #ccdcbc;
  --notification-soft: #2d362a;
}

[data-theme='dark'] .notification-eye_care {
  --action-bg: #294440;
  --action-hover: #345550;
  --action-text: #b7dbd5;
  --notification-accent: #8bbdb4;
  --notification-ink: #b7dbd5;
  --notification-soft: #263834;
}

.notification-custom {
  --action-bg: #ebe7f2;
  --action-hover: #dfd8eb;
  --action-text: #635575;
  --notification-accent: #a095b2;
  --notification-ink: #635575;
  --notification-soft: #f4f1f7;
}

[data-theme='dark'] .notification-custom {
  --action-bg: #40384d;
  --action-hover: #514660;
  --action-text: #d7cbe7;
  --notification-accent: #b7a6cc;
  --notification-ink: #d7cbe7;
  --notification-soft: #332e3c;
}

.notification-header {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 20px;
  flex-shrink: 0;
}

.notification-tag {
  flex-shrink: 0;
  border-radius: 6px;
  padding: 2px 7px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: var(--notification-soft);
  color: var(--notification-ink);
  font-size: 10px;
  font-weight: 600;
  line-height: 16px;
}

.notification-tag::before {
  content: '';
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--notification-accent);
}

.notification-queue {
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  color: var(--text-secondary);
  font-size: 10px;
}

.skip-button {
  margin-left: auto;
  flex-shrink: 0;
  padding: 2px 0 2px 4px;
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 16px;
  background: transparent;
  border: 0;
}

.skip-button:hover {
  color: var(--notification-ink);
  text-decoration: underline;
  text-underline-offset: 3px;
}

.notification-body {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 80px minmax(0, 1fr);
  align-items: center;
  gap: 12px;
}

.notification-visual {
  display: flex;
  align-items: center;
  justify-content: center;
}

.notification-image {
  width: 72px;
  height: 72px;
  object-fit: contain;
}

.notification-visual-mascot .notification-image {
  width: 88px;
  height: 100px;
}

.notification-fallback-icon {
  font-size: 28px;
  color: var(--notification-ink);
}

.notification-content {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.notification-custom .notification-body {
  grid-template-columns: 64px minmax(0, 1fr);
}

.notification-custom .notification-image {
  width: 56px;
  height: 56px;
}

.notification-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  line-height: 1.2;
  color: var(--text-primary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  overflow-wrap: anywhere;
}

.notification-message {
  margin: 0;
  font-size: 12px;
  line-height: 1.4;
  color: var(--text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  overflow-wrap: anywhere;
}

.complete-button {
  min-height: 28px;
  padding: 4px 12px;
  border: 0;
  border-radius: 9px;
  background: var(--action-bg);
  color: var(--action-text);
  font-family: inherit;
  font-size: 12px;
  line-height: 20px;
  font-weight: 600;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  transition: filter 150ms ease, transform 150ms ease;
}

.complete-button:hover { background: var(--action-hover); }
.complete-button:active, .postpone-button:active { transform: translateY(1px); }

.postpone-grid {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  border-top: 1px solid var(--border-color);
  padding-top: 7px;
}

.postpone-label {
  flex-shrink: 0;
  font-size: 10px;
  color: var(--text-secondary);
}

.postpone-options {
  display: flex;
  flex: 1;
  min-width: 0;
  gap: 4px;
  overflow-x: auto;
}

.postpone-button {
  flex: 1 0 auto;
  padding: 3px 6px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-family: inherit;
  font-size: 11px;
  line-height: 18px;
  white-space: nowrap;
  transition: background-color 150ms ease;
}

.postpone-button:hover {
  color: var(--notification-ink);
  background: var(--bg-secondary);
}

.countdown-row {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.break-countdown {
  font-size: 30px;
  line-height: 1.1;
  font-weight: 500;
  color: var(--notification-ink);
  letter-spacing: -0.04em;
  font-variant-numeric: tabular-nums;
}

.countdown-label {
  font-size: 10px;
  color: var(--text-secondary);
}

.countdown-footer {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-shrink: 0;
}

.countdown-track {
  flex: 1;
  height: 4px;
  overflow: hidden;
  border-radius: 4px;
  background: var(--bg-secondary);
}

.countdown-fill {
  height: 100%;
  background: var(--notification-accent);
  transform-origin: left;
  transition: transform 250ms linear;
}

.finish-button { max-width: 70%; }

button:focus-visible {
  outline: 2px solid var(--notification-ink);
  outline-offset: 2px;
}

@media (prefers-reduced-motion: reduce) {
  .notification-wrapper-visible { animation: none; }
  .complete-button, .postpone-button, .countdown-fill { transition: none; }
}

@keyframes slide-in {
  from { transform: translateY(8px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
</style>
