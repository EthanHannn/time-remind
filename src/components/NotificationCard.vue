<script setup lang="ts">
import type { NotificationAppearance } from '../utils/notificationAppearance'
import type { ResolvedNotificationPalette } from '../utils/notificationPalette'
import type { ReminderVisual } from '../utils/reminderVisuals'
import { computed, shallowRef, watch } from 'vue'
import { useI18n } from '../i18n'
import { appIconMain } from '../utils/reminderVisuals'
import AppIcon from './AppIcon.vue'
import CountdownProgress from './CountdownProgress.vue'
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
  palette?: ResolvedNotificationPalette
  collapsed?: boolean
  completed?: boolean
  suppressed?: boolean
  actionEnabled?: boolean
  remainingProgress?: number
  remainingSeconds?: number
  countdownRunning?: boolean
  contentHeight?: number
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
watch([() => props.visible, () => props.suppressed], ([visible, suppressed]) => {
  if (!visible || suppressed) {
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
const headerTagLabel = computed(() => {
  if (props.preview)
    return t('settings.previewBadge')
  return props.visual.type === 'custom' ? t('common.appName') : props.visual.label
})
const countdownMessage = computed(() => props.actionMessage || (props.visual.type === 'eye_care' ? t('notification.eyeHint') : props.visual.type === 'rest' ? t('notification.restHint') : props.message))
const primaryAsset = computed(() => props.visual.mascotAsset || props.visual.iconAsset)
const statusAsset = computed(() => props.visual.statusAsset || primaryAsset.value)
const showStatusAsset = computed(() => Boolean(props.breakMode && props.visual.type !== 'eye_care' && statusAsset.value))
const hasDistinctStatusAsset = computed(() => Boolean(statusAsset.value && statusAsset.value !== primaryAsset.value))
const compactAsset = computed(() => showStatusAsset.value ? statusAsset.value : primaryAsset.value || props.visual.iconAsset)
const progress = computed(() => Math.max(0, Math.min(1, props.remainingProgress ?? 1)))
</script>

<template>
  <div
    class="notification-wrapper" :class="{ 'notification-wrapper-visible': visible, 'notification-wrapper-compact': collapsed && breakMode }"
    :style="contentHeight ? { '--notification-shell-height': `calc(${contentHeight}px - 2 * var(--notification-padding))` } : undefined"
  >
    <section
      v-if="visible" class="notification-shell"
      :class="[`notification-${visual.type}`, `appearance-${appearance || 'soft'}`, `palette-${palette || 'paper'}`, { 'notification-counting': breakMode, 'notification-collapsed': collapsed && breakMode }]"
      :aria-label="preview ? t('settings.previewReminder') : visual.label"
      @mouseenter="hovered = true" @mouseleave="hovered = false" @focusin="focused = true" @focusout="focusOut"
    >
      <template v-if="collapsed && breakMode">
        <img v-if="compactAsset" :src="compactAsset" alt="" class="compact-icon" width="36" height="36" decoding="sync">
        <div class="compact-copy">
          <span class="compact-title" :title="actionTitle" :role="completed ? 'status' : undefined">{{ actionTitle }}</span>
          <div v-if="!completed" class="compact-track" aria-hidden="true">
            <CountdownProgress class="countdown-fill" :value="progress" :target="0" :remaining-seconds="remainingSeconds || 0" :running="Boolean(countdownRunning)" />
          </div>
        </div>
        <AppIcon v-if="completed" name="check" class="completion-check" />
        <span v-else class="compact-countdown" role="timer" :aria-label="t('notification.remaining')"><bdi dir="ltr">{{ breakCountdownLabel }}</bdi></span>
        <button v-if="!completed" class="compact-expand" type="button" :aria-label="t('ui.expand')" :title="t('ui.expand')" @click="emit('collapse', false)">
          <AppIcon name="expand" />
        </button>
        <p v-if="error" class="notification-error" role="alert">
          {{ error }}
        </p>
      </template>
      <template v-else>
        <header class="notification-header">
          <img v-if="appearance === 'minimal'" :src="appIconMain" alt="" width="16" height="16">
          <span class="notification-tag" :class="{ 'notification-tag-counting': breakMode && !completed }">
            {{ headerTagLabel }}
            <template v-if="breakMode && !completed"> · {{ t('ui.countingTag') }}</template>
          </span>
          <span v-if="pendingLabel" class="notification-queue" :title="pendingLabel">{{ pendingLabel }}</span>
          <button v-if="!breakMode" class="skip-button" :disabled="pending" type="button" @click="emit('skip')">
            {{ t('notification.skip') }}
          </button>
        </header>
        <div v-if="completed" class="notification-completion" role="status">
          <span class="completion-icon"><AppIcon name="check" /></span>
          <h2 class="notification-title">
            {{ actionTitle }}
          </h2>
          <p class="notification-message">
            {{ actionMessage }}
          </p>
        </div>
        <div v-else class="notification-body">
          <div v-if="appearance !== 'minimal'" class="notification-visual">
            <template v-if="primaryAsset">
              <img
                :src="primaryAsset"
                alt=""
                class="notification-image-layer"
                :class="showStatusAsset && hasDistinctStatusAsset ? 'notification-image-hidden' : 'notification-image'"
                decoding="sync"
                width="112"
                height="112"
              >
              <img
                v-if="hasDistinctStatusAsset"
                :src="statusAsset"
                alt=""
                class="notification-image-layer"
                :class="showStatusAsset ? 'notification-image' : 'notification-image-hidden'"
                decoding="sync"
                width="112"
                height="112"
              >
            </template>
            <span v-else class="notification-fallback-icon" aria-hidden="true">{{ visual.iconText }}</span>
          </div>
          <div class="notification-content">
            <h2 class="notification-title" :title="breakMode ? actionTitle : name">
              {{ breakMode ? actionTitle : name }}
            </h2>
            <p class="notification-message" :title="breakMode ? countdownMessage : message">
              {{ breakMode ? countdownMessage : message }}
            </p>
            <div v-if="breakMode" class="countdown-block">
              <div class="countdown-row">
                <span class="break-countdown" role="timer" :aria-label="t('notification.remaining')"><bdi dir="ltr">{{ breakCountdownLabel }}</bdi></span>
                <span class="countdown-label">{{ t('notification.remaining') }}</span>
              </div>
              <div class="countdown-track" aria-hidden="true">
                <CountdownProgress class="countdown-fill" :value="progress" :target="0" :remaining-seconds="remainingSeconds || 0" :running="Boolean(countdownRunning)" />
              </div>
            </div>
          </div>
        </div>
        <p v-if="error" class="notification-error" role="alert">
          {{ error }}
        </p>
        <footer v-if="!completed" class="notification-actions">
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
.notification-wrapper {
  --notification-padding: 12px;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: flex-end;
  padding: var(--notification-padding);
  background: transparent;
}
.notification-wrapper-compact {
  --notification-padding: 8px;
}
.notification-wrapper-visible {
  animation: slide-in 190ms cubic-bezier(0.22, 1, 0.36, 1);
}

/* ===== Base Shell ===== */
.notification-shell {
  width: 100%;
  height: var(--notification-shell-height, 100%);
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
  border-radius: 20px;
  border: 1px solid var(--popup-border);
  background: var(--popup-surface);
  color: var(--popup-text);
  box-shadow: var(--popup-shadow);
  transition: background 180ms ease, border-color 180ms ease;
}

/* Keep button visually steady during fast IPC roundtrips so clicking Start Rest never blinks gray */
.notification-shell :deep(button:disabled) {
  opacity: 1;
  cursor: default;
}

/* ===== Palette 1: Daylight Peach (晨光杏桃 · paper) ===== */
.palette-paper {
  --popup-surface:
    radial-gradient(circle at top right, rgb(229 103 71 / 10%), transparent 54%),
    linear-gradient(155deg, #ffffff 0%, #fdf8f5 100%);
  --popup-surface-solid: #fffdfb;
  --popup-sub-bg: #f8f1ec;
  --popup-border: rgb(229 103 71 / 20%);
  --popup-text: #221815;
  --popup-muted: #766159;
  --popup-cta-bg: #e56747;
  --popup-cta-hover: #d45738;
  --popup-cta-text: #ffffff;
  --popup-sec-bg: #f8f1ec;
  --popup-sec-text: #3d2c26;
  --popup-accent: #e56747;
  --popup-halo: rgb(229 103 71 / 11%);
  --popup-shadow: 0 12px 30px rgb(96 44 28 / 12%), 0 2px 6px rgb(96 44 28 / 6%), inset 0 1px 0 rgb(255 255 255 / 92%);
}
.palette-paper.notification-drink { --popup-accent: #357fc4; --popup-halo: rgb(53 127 196 / 11%); }
.palette-paper.notification-rest { --popup-accent: #28966e; --popup-halo: rgb(40 150 110 / 11%); }
.palette-paper.notification-eye_care { --popup-accent: #de7c32; --popup-halo: rgb(222 124 50 / 12%); }

[data-theme='dark'] .palette-paper {
  --popup-surface:
    radial-gradient(circle at top right, rgb(243 141 113 / 15%), transparent 54%),
    linear-gradient(155deg, #27201f 0%, #1b1615 100%);
  --popup-surface-solid: #211b1a;
  --popup-sub-bg: #2e2524;
  --popup-border: rgb(243 141 113 / 24%);
  --popup-text: #faf2ee;
  --popup-muted: #b59e96;
  --popup-cta-bg: #f38d71;
  --popup-cta-hover: #f7a088;
  --popup-cta-text: #23130e;
  --popup-sec-bg: #2d2423;
  --popup-sec-text: #efe3dd;
  --popup-accent: #f38d71;
  --popup-halo: rgb(243 141 113 / 16%);
  --popup-shadow: 0 14px 34px rgb(0 0 0 / 50%), 0 2px 8px rgb(0 0 0 / 30%), inset 0 1px 0 rgb(255 235 225 / 8%);
}
[data-theme='dark'] .palette-paper.notification-drink { --popup-accent: #69ade8; --popup-halo: rgb(105 173 232 / 15%); }
[data-theme='dark'] .palette-paper.notification-rest { --popup-accent: #4ecb9b; --popup-halo: rgb(78 203 155 / 15%); }
[data-theme='dark'] .palette-paper.notification-eye_care { --popup-accent: #f3a661; --popup-halo: rgb(243 166 97 / 16%); }

/* ===== Palette 2: Linear Aurora (极光星曜 · ink) ===== */
.palette-ink {
  --popup-surface:
    radial-gradient(circle at top right, rgb(94 92 230 / 10%), transparent 55%),
    linear-gradient(155deg, #ffffff 0%, #f8f7fd 100%);
  --popup-surface-solid: #fdfcff;
  --popup-sub-bg: #f3f1fa;
  --popup-border: rgb(94 92 230 / 18%);
  --popup-text: #161426;
  --popup-muted: #64617e;
  --popup-cta-bg: #5e5ce6;
  --popup-cta-hover: #4e4cd8;
  --popup-cta-text: #ffffff;
  --popup-sec-bg: #f3f1fa;
  --popup-sec-text: #282542;
  --popup-accent: #5e5ce6;
  --popup-halo: rgb(94 92 230 / 11%);
  --popup-shadow: 0 12px 30px rgb(48 38 108 / 12%), 0 2px 6px rgb(48 38 108 / 6%), inset 0 1px 0 rgb(255 255 255 / 92%);
}
.palette-ink.notification-drink { --popup-accent: #3f7cf0; --popup-halo: rgb(63 124 240 / 11%); }
.palette-ink.notification-rest { --popup-accent: #1ca384; --popup-halo: rgb(28 163 132 / 11%); }
.palette-ink.notification-eye_care { --popup-accent: #d98236; --popup-halo: rgb(217 130 54 / 12%); }

[data-theme='dark'] .palette-ink {
  --popup-surface:
    radial-gradient(circle at top right, rgb(139 133 255 / 17%), transparent 55%),
    linear-gradient(155deg, #1b1d2b 0%, #13141f 100%);
  --popup-surface-solid: #171924;
  --popup-sub-bg: #232638;
  --popup-border: rgb(139 133 255 / 26%);
  --popup-text: #f4f4fa;
  --popup-muted: #9899b3;
  --popup-cta-bg: #8b85ff;
  --popup-cta-hover: #9d98ff;
  --popup-cta-text: #111024;
  --popup-sec-bg: #222536;
  --popup-sec-text: #e2e3f2;
  --popup-accent: #8b85ff;
  --popup-halo: rgb(139 133 255 / 17%);
  --popup-shadow: 0 16px 36px rgb(0 0 0 / 54%), 0 2px 8px rgb(0 0 0 / 34%), inset 0 1px 0 rgb(255 255 255 / 9%);
}
[data-theme='dark'] .palette-ink.notification-drink { --popup-accent: #6ea8ff; --popup-halo: rgb(110 168 255 / 16%); }
[data-theme='dark'] .palette-ink.notification-rest { --popup-accent: #3ad4a8; --popup-halo: rgb(58 212 168 / 16%); }
[data-theme='dark'] .palette-ink.notification-eye_care { --popup-accent: #f0ab64; --popup-halo: rgb(240 171 100 / 16%); }

/* ===== Palette 3: Cloud Porcelain (云境冰瓷 · sage) ===== */
.palette-sage {
  --popup-surface:
    radial-gradient(circle at top right, rgb(59 102 245 / 8%), transparent 54%),
    linear-gradient(155deg, #ffffff 0%, #f6f9fd 100%);
  --popup-surface-solid: #ffffff;
  --popup-sub-bg: #f1f5fb;
  --popup-border: rgb(59 102 245 / 16%);
  --popup-text: #0f172a;
  --popup-muted: #5d6a82;
  --popup-cta-bg: #3b66f5;
  --popup-cta-hover: #2c55e3;
  --popup-cta-text: #ffffff;
  --popup-sec-bg: #f1f5fb;
  --popup-sec-text: #1e293b;
  --popup-accent: #3b66f5;
  --popup-halo: rgb(59 102 245 / 11%);
  --popup-shadow: 0 12px 30px rgb(24 42 88 / 12%), 0 2px 6px rgb(24 42 88 / 6%), inset 0 1px 0 rgb(255 255 255 / 94%);
}
.palette-sage.notification-drink { --popup-accent: #2e7cf6; --popup-halo: rgb(46 124 246 / 11%); }
.palette-sage.notification-rest { --popup-accent: #10a37f; --popup-halo: rgb(16 163 127 / 11%); }
.palette-sage.notification-eye_care { --popup-accent: #dc8232; --popup-halo: rgb(220 130 50 / 12%); }

[data-theme='dark'] .palette-sage {
  --popup-surface:
    radial-gradient(circle at top right, rgb(109 140 255 / 14%), transparent 54%),
    linear-gradient(155deg, #1b2234 0%, #131826 100%);
  --popup-surface-solid: #171c2b;
  --popup-sub-bg: #20273b;
  --popup-border: rgb(109 140 255 / 24%);
  --popup-text: #f1f5ff;
  --popup-muted: #94a3c4;
  --popup-cta-bg: #6d8cff;
  --popup-cta-hover: #839eff;
  --popup-cta-text: #0c1222;
  --popup-sec-bg: #20273b;
  --popup-sec-text: #e2e8f8;
  --popup-accent: #6d8cff;
  --popup-halo: rgb(109 140 255 / 16%);
  --popup-shadow: 0 16px 36px rgb(0 0 0 / 52%), 0 2px 8px rgb(0 0 0 / 32%), inset 0 1px 0 rgb(255 255 255 / 8%);
}
[data-theme='dark'] .palette-sage.notification-drink { --popup-accent: #60a5fa; --popup-halo: rgb(96 165 250 / 16%); }
[data-theme='dark'] .palette-sage.notification-rest { --popup-accent: #34d399; --popup-halo: rgb(52 211 153 / 16%); }
[data-theme='dark'] .palette-sage.notification-eye_care { --popup-accent: #f4a85c; --popup-halo: rgb(244 168 92 / 16%); }

/* ===== Header ===== */
.notification-header { display: flex; align-items: center; gap: 7px; min-height: 22px; flex-shrink: 0; }
.notification-tag { display: inline-flex; align-items: center; gap: 6px; min-width: 0; color: var(--popup-muted); font-size: 11px; font-weight: 500; line-height: 1.4; }
.notification-tag::before { content: ''; width: 7px; height: 7px; flex-shrink: 0; border-radius: 50%; background: var(--popup-accent); }
.notification-tag-counting::before { animation: pulse-dot 1.6s ease-in-out infinite; }
.notification-queue { min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: var(--popup-muted); font-size: 10px; }
.skip-button { margin-inline-start: auto; padding: 2px 6px; min-height: 24px; flex-shrink: 0; border-radius: 6px; color: var(--popup-muted); font-size: 11px; transition: color 140ms ease, background-color 140ms ease; }
.skip-button:hover { color: var(--popup-text); background: var(--popup-sub-bg); }

/* ===== Body & Preloaded Crossfade Visual ===== */
.notification-body { flex: 1; min-height: 0; display: grid; grid-template-columns: 76px minmax(0, 1fr); align-items: center; gap: 14px; }
.notification-visual { position: relative; display: grid; place-items: center; width: 76px; height: 76px; border-radius: 16px; background: var(--popup-halo); }
.notification-image-layer {
  position: absolute;
  inset: 4px;
  width: calc(100% - 8px);
  height: calc(100% - 8px);
  object-fit: contain;
  transition: opacity 220ms cubic-bezier(0.22, 1, 0.36, 1), transform 240ms cubic-bezier(0.22, 1, 0.36, 1);
  will-change: opacity, transform;
}
.notification-image { opacity: 1; transform: scale(1); }
.notification-image-hidden { opacity: 0; transform: scale(0.94); pointer-events: none; }
.notification-fallback-icon { font-size: 28px; color: var(--popup-accent); }

.notification-content { min-width: 0; display: flex; flex-direction: column; justify-content: center; gap: 5px; }
.notification-title { margin: 0; font-size: 17px; font-weight: 700; line-height: 1.28; letter-spacing: -0.01em; color: var(--popup-text); }
.notification-message { margin: 0; font-size: 12px; line-height: 1.5; color: var(--popup-muted); }
.notification-title, .notification-message { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; overflow-wrap: anywhere; }
.notification-counting .notification-message { -webkit-line-clamp: 1; }
.notification-error { color: var(--color-danger, #bf4154); font-size: 11px; overflow-wrap: anywhere; max-height: 42px; overflow-y: auto; }

/* ===== Integrated Countdown Block ===== */
.countdown-block { display: flex; flex-direction: column; gap: 6px; padding-top: 2px; }
.countdown-row { display: flex; align-items: baseline; gap: 8px; }
.break-countdown { font-size: 25px; line-height: 1.05; font-weight: 700; color: var(--popup-accent); letter-spacing: -0.03em; font-variant-numeric: tabular-nums; }
.countdown-label { font-size: 11px; color: var(--popup-muted); }
.countdown-track { height: 5px; width: 100%; flex-shrink: 0; overflow: hidden; border-radius: 999px; background: var(--popup-sub-bg); }
.countdown-fill { height: 100%; border-radius: 999px; background: var(--popup-accent); transform-origin: left; }

/* ===== Actions ===== */
.notification-actions { display: flex; align-items: stretch; gap: 8px; flex-shrink: 0; }
.notification-actions > * { min-width: 0; }
.complete-button { flex: 1; min-height: 36px; padding: 7px 12px; border-radius: 11px; background: var(--popup-cta-bg); color: var(--popup-cta-text); font-size: 12px; font-weight: 600; line-height: 1.4; overflow-wrap: anywhere; transition: background-color 140ms ease, transform 120ms ease; }
.complete-button:hover { background: var(--popup-cta-hover); }
.complete-button:active { transform: scale(0.99); }
.collapse-button { padding: 6px 12px; min-height: 36px; font-size: 12px; font-weight: 500; border: 1px solid var(--popup-border); border-radius: 11px; background: var(--popup-sec-bg); color: var(--popup-sec-text); transition: background-color 140ms ease, color 140ms ease; }
.collapse-button:hover { background: var(--popup-sub-bg); color: var(--popup-text); }

/* ===== Appearance Variants ===== */
.appearance-minimal { background: var(--popup-surface-solid); border-inline-start: 3.5px solid var(--popup-accent); border-radius: 14px; }
.appearance-minimal .notification-tag::before { display: none; }
.appearance-minimal .notification-body { display: flex; align-items: center; }
.appearance-minimal .notification-content { width: 100%; }

.appearance-illustrated { background: var(--popup-surface); }
.appearance-illustrated .notification-body { grid-template-columns: 112px minmax(0, 1fr); gap: 14px; }
.appearance-illustrated .notification-visual { width: 112px; height: 112px; border-radius: 18px; }
.appearance-illustrated .notification-image-layer { inset: 2px; width: calc(100% - 4px); height: calc(100% - 4px); }
.appearance-illustrated .notification-title { font-size: 18px; }

/* ===== Compact Capsule Pill ===== */
.notification-collapsed {
  flex-direction: row;
  align-items: center;
  flex-wrap: nowrap;
  gap: 10px;
  padding: 10px 14px;
  border-radius: 999px;
}
.compact-icon { width: 38px; height: 38px; flex-shrink: 0; object-fit: contain; }
.compact-copy { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 5px; }
.compact-title { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; font-weight: 600; color: var(--popup-text); }
.compact-track { height: 4px; width: 100%; overflow: hidden; border-radius: 999px; background: var(--popup-sub-bg); }
.compact-countdown { font-size: 16px; font-weight: 700; font-variant-numeric: tabular-nums; color: var(--popup-accent); flex-shrink: 0; }
.compact-expand { display: grid; place-items: center; width: 30px; height: 30px; flex-shrink: 0; border-radius: 999px; background: var(--popup-sub-bg); color: var(--popup-muted); transition: background-color 140ms ease, color 140ms ease; }
.compact-expand:hover { color: var(--popup-text); }

/* ===== Completion View ===== */
.notification-completion { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; text-align: center; padding-bottom: 10px; }
.completion-icon { display: grid; place-items: center; width: 44px; height: 44px; border-radius: 50%; background: var(--popup-halo); color: var(--popup-accent); }
.completion-check { color: var(--popup-accent); }

[dir='rtl'] .countdown-fill { transform-origin: right; }
@media (max-width: 350px) {
  .notification-wrapper { --notification-padding: 8px; }
  .notification-shell { padding: 12px; }
  .appearance-illustrated .notification-body { grid-template-columns: 88px minmax(0, 1fr); gap: 12px; }
  .appearance-illustrated .notification-visual { width: 88px; height: 88px; }
  .appearance-illustrated .notification-title { font-size: 16px; }
}
@keyframes slide-in { from { transform: translateY(6px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
@keyframes pulse-dot { 0%, 100% { transform: scale(1); opacity: 1; } 50% { transform: scale(1.35); opacity: 0.5; } }
</style>
