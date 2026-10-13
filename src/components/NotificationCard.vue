<script setup lang="ts">
import type { NotificationAppearance } from '../utils/notificationAppearance'
import type { ReminderVisual } from '../utils/reminderVisuals'
import { computed, shallowRef, watch } from 'vue'
import { useI18n } from '../i18n'
import { appIconMain } from '../utils/reminderVisuals'
import AppIcon from './AppIcon.vue'
import SnoozeControl from './SnoozeControl.vue'

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
  appearance?: NotificationAppearance
  collapsed?: boolean
  actionEnabled?: boolean
  remainingProgress?: number
  preview?: boolean
  pending?: boolean
  error?: string
}>()
const emit = defineEmits<{
  complete: []
  skip: []
  postpone: [minutes: number]
  finish: []
  collapse: [value: boolean]
  interaction: [active: boolean]
}>()
const { t } = useI18n()
const hovered = shallowRef(false)
const focused = shallowRef(false)
watch(() => props.visible, (visible) => {
  if (!visible) {
    hovered.value = false
    focused.value = false
  }
})
watch(() => props.visible && (hovered.value || focused.value), value => emit('interaction', value))
function focusOut(event: FocusEvent) {
  focused.value = (event.currentTarget as HTMLElement).contains(event.relatedTarget as Node | null)
}
const primaryLabel = computed(() => {
  if (props.actionEnabled) {
    if (props.visual.type === 'eye_care')
      return t('notification.startEyeCare')
    if (props.visual.type === 'rest')
      return t('notification.startBreak')
    return props.actionTitle || t('ui.startAction')
  }
  return t(props.visual.type === 'drink' ? 'notification.drank' : 'notification.complete')
})
const countdownMessage = computed(() => props.actionMessage || (props.visual.type === 'eye_care' ? t('notification.eyeHint') : props.visual.type === 'rest' ? t('notification.restHint') : props.message))
const imageAsset = computed(() => props.breakMode && props.visual.type !== 'eye_care'
  ? props.visual.statusAsset || props.visual.mascotAsset || props.visual.iconAsset
  : props.visual.mascotAsset || props.visual.iconAsset)
const progress = computed(() => Math.max(0, Math.min(1, props.remainingProgress ?? 1)))
</script>

<template>
  <div class="notification-wrapper" :class="{ 'notification-wrapper-visible': visible }">
    <section
      v-if="visible" class="notification-shell"
      :class="[`notification-${visual.type}`, `appearance-${appearance || 'soft'}`, { 'notification-counting': breakMode, 'notification-collapsed': collapsed && breakMode }]"
      :aria-label="preview ? t('settings.previewReminder') : visual.label"
      @mouseenter="hovered = true" @mouseleave="hovered = false" @focusin="focused = true" @focusout="focusOut"
    >
      <template v-if="collapsed && breakMode">
        <img v-if="visual.iconAsset" :src="visual.iconAsset" alt="" class="compact-icon" width="28" height="28">
        <span class="compact-title" :title="actionTitle">{{ actionTitle }}</span>
        <span class="compact-countdown" role="timer" :aria-label="t('notification.remaining')"><bdi dir="ltr">{{ breakCountdownLabel }}</bdi></span>
        <button class="compact-expand" type="button" :aria-label="t('ui.expand')" :title="t('ui.expand')" @click="emit('collapse', false)">
          <AppIcon name="expand" />
        </button>
        <p v-if="error" class="notification-error" role="alert">
          {{ error }}
        </p>
      </template>
      <template v-else>
        <header class="notification-header">
          <img v-if="appearance === 'minimal'" :src="appIconMain" alt="" width="16" height="16">
          <span class="notification-tag">{{ preview ? t('settings.previewBadge') : visual.label }}</span>
          <span v-if="pendingLabel" class="notification-queue" :title="pendingLabel">{{ pendingLabel }}</span>
          <button v-if="!breakMode" class="skip-button" :disabled="pending" type="button" @click="emit('skip')">
            {{ t('notification.skip') }}
          </button>
        </header>
        <div class="notification-body">
          <div v-if="appearance !== 'minimal'" class="notification-visual">
            <img v-if="imageAsset" :src="imageAsset" alt="" class="notification-image" width="112" height="112">
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
              <span class="break-countdown" role="timer" :aria-label="t('notification.remaining')"><bdi dir="ltr">{{ breakCountdownLabel }}</bdi></span>
              <span class="countdown-label">{{ t('notification.remaining') }}</span>
            </div>
          </div>
        </div>
        <p v-if="error" class="notification-error" role="alert">
          {{ error }}
        </p>
        <div v-if="breakMode" class="countdown-track" aria-hidden="true">
          <div class="countdown-fill" :style="{ transform: `scaleX(${progress})` }" />
        </div>
        <footer class="notification-actions">
          <template v-if="breakMode">
            <button class="complete-button finish-button" :disabled="pending" type="button" @click="emit('finish')">
              {{ t(visual.type === 'rest' ? 'notification.finishBreak' : 'notification.complete') }}
            </button>
            <button class="collapse-button" :disabled="pending" type="button" @click="emit('collapse', true)">
              {{ t('ui.collapse') }}
            </button>
          </template>
          <template v-else>
            <button class="complete-button" :disabled="pending" type="button" @click="emit('complete')">
              {{ primaryLabel }}
            </button>
            <SnoozeControl :options="postponeOptions" :disabled="pending" @postpone="emit('postpone', $event)" />
          </template>
        </footer>
      </template>
    </section>
  </div>
</template>

<style scoped>
.notification-wrapper { width: 100%; height: 100%; display: flex; padding: 16px; background: transparent; }
.notification-wrapper-visible { animation: slide-in 200ms ease-out; }
.notification-shell {
  --action-bg: #e7e1ee; --action-hover: #ded6e7; --action-text: #59496c;
  --notification-accent: #a095b2; --notification-soft: #f6f3f8;
  width: 100%; height: 100%; min-height: 0; display: flex; flex-direction: column; gap: 10px;
  padding: 14px 16px; border-radius: 18px; border: 1px solid var(--border-color);
  background: var(--bg-primary); box-shadow: 0 3px 12px rgb(20 40 60 / 7%);
}
.notification-drink { --action-bg: #ddeaf1; --action-hover: #ccdfeb; --action-text: #30536c; --notification-accent: #8aabba; --notification-soft: #f1f6f9; }
.notification-rest { --action-bg: #e1ede6; --action-hover: #d3e5dc; --action-text: #3b6250; --notification-accent: #90ac9d; --notification-soft: #f2f7f4; }
.notification-eye_care { --action-bg: #f3e8d9; --action-hover: #ecddc8; --action-text: #725631; --notification-accent: #c2a57b; --notification-soft: #fbf6ef; }
[data-theme='dark'] .notification-shell { --action-bg: #453d53; --action-hover: #534963; --action-text: #ddd1ea; --notification-soft: #302d3c; box-shadow: 0 4px 12px rgb(2 6 23 / 24%); }
[data-theme='dark'] .notification-drink { --action-bg: #304957; --action-hover: #3c5868; --action-text: #c7e0ed; --notification-soft: #233541; }
[data-theme='dark'] .notification-rest { --action-bg: #344d41; --action-hover: #425e50; --action-text: #cee6d9; --notification-soft: #273a32; }
[data-theme='dark'] .notification-eye_care { --action-bg: #514535; --action-hover: #63543f; --action-text: #eed9b8; --notification-soft: #3b3328; }
.notification-header { display: flex; align-items: center; gap: 7px; min-height: 24px; flex-shrink: 0; }
.notification-tag { display: inline-flex; align-items: center; gap: 6px; min-width: 0; color: var(--text-secondary); font-size: 11px; line-height: 1.4; }
.notification-tag::before { content: ''; width: 6px; height: 6px; flex-shrink: 0; border-radius: 50%; background: var(--notification-accent); }
.notification-queue { min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: var(--text-secondary); font-size: 10px; }
.skip-button { margin-inline-start: auto; padding: 4px; min-height: 28px; flex-shrink: 0; color: var(--text-secondary); font-size: 11px; }
.skip-button:hover { color: var(--text-primary); text-decoration: underline; text-underline-offset: 3px; }
.notification-body { flex: 1; min-height: 0; display: grid; grid-template-columns: 54px minmax(0, 1fr); align-items: center; gap: 12px; }
.notification-visual { display: grid; place-items: center; width: 54px; height: 54px; border-radius: 12px; background: var(--notification-soft); }
.notification-image { width: 50px; height: 50px; object-fit: contain; }
.notification-fallback-icon { font-size: 28px; color: var(--action-text); }
.notification-content { min-width: 0; display: flex; flex-direction: column; gap: 6px; }
.notification-title { margin: 0; font-size: 17px; font-weight: 650; line-height: 1.3; color: var(--text-primary); }
.notification-message { margin: 0; font-size: 12px; line-height: 1.55; color: var(--text-secondary); }
.notification-title, .notification-message { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; overflow-wrap: anywhere; }
.notification-error { color: var(--color-danger, #bf4154); font-size: 11px; overflow-wrap: anywhere; max-height: 42px; overflow-y: auto; }
.notification-actions { display: flex; align-items: stretch; gap: 8px; flex-shrink: 0; }
.notification-actions > * { min-width: 0; }
.complete-button { flex: 1; min-height: 36px; padding: 7px 10px; border-radius: 9px; background: var(--action-bg); color: var(--action-text); font-size: 12px; font-weight: 600; line-height: 1.45; overflow-wrap: anywhere; }
.complete-button:hover { background: var(--action-hover); }
.collapse-button { padding: 6px 10px; font-size: 12px; border: 1px solid var(--border-color); border-radius: 9px; color: var(--text-secondary); }
.collapse-button:hover { color: var(--text-primary); background: var(--bg-secondary); }
.countdown-row { display: flex; align-items: baseline; gap: 8px; }
.break-countdown { font-size: 26px; line-height: 1.1; font-weight: 600; color: var(--action-text); letter-spacing: -0.03em; font-variant-numeric: tabular-nums; }
.countdown-label { font-size: 10px; color: var(--text-secondary); }
.countdown-track { height: 4px; flex-shrink: 0; overflow: hidden; border-radius: 4px; background: var(--bg-secondary); }
.countdown-fill { height: 100%; background: var(--notification-accent); transform-origin: left; transition: transform 250ms linear; }
.appearance-minimal { border-inline-start: 3px solid var(--notification-accent); border-radius: 12px; }
.appearance-minimal .notification-tag::before { display: none; }
.appearance-minimal .notification-body { display: block; align-content: center; }
.appearance-illustrated { padding: 14px 16px; gap: 8px; background: linear-gradient(145deg, var(--bg-primary), var(--notification-soft)); }
.appearance-illustrated .notification-body { grid-template-columns: 104px minmax(0, 1fr); gap: 12px; }
.appearance-illustrated .notification-visual { width: 104px; height: 108px; background: none; border-radius: 0; }
.appearance-illustrated .notification-image { width: 104px; height: 108px; }
.appearance-illustrated .notification-title { font-size: 20px; }
.notification-collapsed { flex-direction: row; align-items: center; flex-wrap: wrap; gap: 8px; padding: 10px 12px; }
.compact-icon { object-fit: contain; }
.compact-title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
.compact-countdown { font-size: 17px; font-weight: 650; font-variant-numeric: tabular-nums; color: var(--action-text); }
.compact-expand { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 7px; color: var(--text-secondary); }
.compact-expand:hover { background: var(--bg-secondary); }
[dir='rtl'] .countdown-fill { transform-origin: right; }
@media (max-width: 350px) {
  .notification-wrapper { padding: 10px; }
  .notification-shell { padding: 12px; }
  .appearance-illustrated .notification-body { grid-template-columns: 80px minmax(0, 1fr); gap: 12px; }
  .appearance-illustrated .notification-visual, .appearance-illustrated .notification-image { width: 80px; height: 100px; }
  .appearance-illustrated .notification-title { font-size: 18px; }
}
@keyframes slide-in { from { transform: translateY(6px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
</style>
