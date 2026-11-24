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
            {{ preview ? t('settings.previewBadge') : visual.label }}
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
  background: var(--popup-surface-solid);
  color: var(--popup-text);
  box-shadow: var(--popup-shadow);
  transition: background 180ms ease, border-color 180ms ease;
}

/* Keep button visually steady during fast IPC roundtrips so clicking Start Rest never blinks gray */
.notification-shell :deep(button:disabled) {
  opacity: 1;
  cursor: default;
}

/* ===== Palette 1: Warm Paper (暖纸奶油) ===== */
.palette-paper {
  --popup-surface: linear-gradient(145deg, #fdfbf6 0%, #f7f0e4 100%);
  --popup-surface-solid: #fcf9f2;
  --popup-sub-bg: #efe7da;
  --popup-border: rgb(110 82 54 / 15%);
  --popup-text: #2a211a;
  --popup-muted: #76675a;
  --popup-cta-bg: #362c24;
  --popup-cta-hover: #493c32;
  --popup-cta-text: #fcf9f2;
  --popup-sec-bg: rgb(110 82 54 / 6%);
  --popup-sec-text: #3d3127;
  --popup-accent: #7f6d90;
  --popup-halo: rgb(127 109 144 / 11%);
  --popup-shadow: 0 8px 24px rgb(62 43 24 / 12%), 0 2px 6px rgb(62 43 24 / 6%);
}
.palette-paper.notification-drink { --popup-accent: #4c839e; --popup-halo: rgb(76 131 158 / 11%); }
.palette-paper.notification-rest { --popup-accent: #558669; --popup-halo: rgb(85 134 105 / 12%); }
.palette-paper.notification-eye_care { --popup-accent: #c27b3c; --popup-halo: rgb(194 123 60 / 12%); }

[data-theme='dark'] .palette-paper {
  --popup-surface: linear-gradient(145deg, #24201c 0%, #1c1815 100%);
  --popup-surface-solid: #211d19;
  --popup-sub-bg: #2e2822;
  --popup-border: rgb(236 218 194 / 14%);
  --popup-text: #f5efe6;
  --popup-muted: #aba092;
  --popup-cta-bg: #e8dcc8;
  --popup-cta-hover: #f2e8d7;
  --popup-cta-text: #221c16;
  --popup-sec-bg: rgb(236 218 194 / 8%);
  --popup-sec-text: #e7dfd3;
  --popup-accent: #b39ec9;
  --popup-halo: rgb(179 158 201 / 14%);
  --popup-shadow: 0 10px 26px rgb(0 0 0 / 42%), 0 2px 6px rgb(0 0 0 / 26%);
}
[data-theme='dark'] .palette-paper.notification-drink { --popup-accent: #7ab4cf; --popup-halo: rgb(122 180 207 / 14%); }
[data-theme='dark'] .palette-paper.notification-rest { --popup-accent: #7db896; --popup-halo: rgb(125 184 150 / 14%); }
[data-theme='dark'] .palette-paper.notification-eye_care { --popup-accent: #dfa263; --popup-halo: rgb(223 162 99 / 15%); }

/* ===== Palette 2: Minimal Ink (极简雾墨) ===== */
.palette-ink {
  --popup-surface: linear-gradient(150deg, #ffffff 0%, #f5f7fa 100%);
  --popup-surface-solid: #fcfcfd;
  --popup-sub-bg: #eef1f6;
  --popup-border: rgb(18 24 38 / 10%);
  --popup-text: #141820;
  --popup-muted: #656e7e;
  --popup-cta-bg: #181c24;
  --popup-cta-hover: #2a303d;
  --popup-cta-text: #ffffff;
  --popup-sec-bg: #f1f3f7;
  --popup-sec-text: #242a36;
  --popup-accent: #756b93;
  --popup-halo: rgb(117 107 147 / 10%);
  --popup-shadow: 0 8px 26px rgb(15 23 42 / 10%), 0 1px 4px rgb(15 23 42 / 5%);
}
.palette-ink.notification-drink { --popup-accent: #5282a8; --popup-halo: rgb(82 130 168 / 10%); }
.palette-ink.notification-rest { --popup-accent: #4e8c74; --popup-halo: rgb(78 140 116 / 10%); }
.palette-ink.notification-eye_care { --popup-accent: #b88142; --popup-halo: rgb(184 129 66 / 11%); }

[data-theme='dark'] .palette-ink {
  --popup-surface: linear-gradient(150deg, #1b1e26 0%, #14171e 100%);
  --popup-surface-solid: #181b22;
  --popup-sub-bg: #252a35;
  --popup-border: rgb(255 255 255 / 11%);
  --popup-text: #f3f5f8;
  --popup-muted: #98a1b2;
  --popup-cta-bg: #eceff5;
  --popup-cta-hover: #ffffff;
  --popup-cta-text: #14171f;
  --popup-sec-bg: #232732;
  --popup-sec-text: #e2e6ee;
  --popup-accent: #a89dc8;
  --popup-halo: rgb(168 157 200 / 14%);
  --popup-shadow: 0 10px 28px rgb(0 0 0 / 48%), 0 2px 8px rgb(0 0 0 / 30%);
}
[data-theme='dark'] .palette-ink.notification-drink { --popup-accent: #7ab0da; --popup-halo: rgb(122 176 218 / 14%); }
[data-theme='dark'] .palette-ink.notification-rest { --popup-accent: #73ba9e; --popup-halo: rgb(115 186 158 / 14%); }
[data-theme='dark'] .palette-ink.notification-eye_care { --popup-accent: #e0ad6e; --popup-halo: rgb(224 173 110 / 15%); }

/* ===== Palette 3: Soft Mist (柔雾森息) ===== */
.palette-sage {
  --popup-surface: linear-gradient(145deg, #faf9fc 0%, #f1eef7 100%);
  --popup-surface-solid: #f8f7fb;
  --popup-sub-bg: rgb(24 42 36 / 6%);
  --popup-border: rgb(24 42 36 / 10%);
  --popup-text: #1d2925;
  --popup-muted: #5a6c65;
  --popup-cta-bg: #e5def0;
  --popup-cta-hover: #dad0ea;
  --popup-cta-text: #45355a;
  --popup-sec-bg: rgb(255 255 255 / 72%);
  --popup-sec-text: #2a3b35;
  --popup-accent: #7c6f97;
  --popup-halo: rgb(124 111 151 / 11%);
  --popup-shadow: 0 8px 24px rgb(20 44 36 / 10%), 0 2px 6px rgb(20 44 36 / 5%);
}
.palette-sage.notification-drink {
  --popup-surface: linear-gradient(145deg, #f9fcfe 0%, #eaf3f8 100%);
  --popup-surface-solid: #f5fafd;
  --popup-accent: #447e9e;
  --popup-halo: rgb(68 126 158 / 11%);
  --popup-cta-bg: #d4e8f3;
  --popup-cta-hover: #c5dfee;
  --popup-cta-text: #1c465e;
}
.palette-sage.notification-rest {
  --popup-surface: linear-gradient(145deg, #f9fcf9 0%, #eaf3ed 100%);
  --popup-surface-solid: #f5faf6;
  --popup-accent: #438568;
  --popup-halo: rgb(67 133 104 / 11%);
  --popup-cta-bg: #d4ebdf;
  --popup-cta-hover: #c5e3d3;
  --popup-cta-text: #1d4b37;
}
.palette-sage.notification-eye_care {
  --popup-surface: linear-gradient(145deg, #fdfbf7 0%, #f6ede0 100%);
  --popup-surface-solid: #fbf8f2;
  --popup-accent: #b57736;
  --popup-halo: rgb(181 119 54 / 11%);
  --popup-cta-bg: #f1dfc6;
  --popup-cta-hover: #ead2b1;
  --popup-cta-text: #5f3c16;
}

[data-theme='dark'] .palette-sage {
  --popup-surface: linear-gradient(145deg, #211d28 0%, #191620 100%);
  --popup-surface-solid: #1e1a25;
  --popup-sub-bg: rgb(255 255 255 / 7%);
  --popup-border: rgb(215 236 228 / 12%);
  --popup-text: #ecf3f0;
  --popup-muted: #98aea6;
  --popup-cta-bg: #3f3452;
  --popup-cta-hover: #4d4063;
  --popup-cta-text: #e4daf4;
  --popup-sec-bg: rgb(255 255 255 / 6%);
  --popup-sec-text: #dce7e2;
  --popup-accent: #aa9bc7;
  --popup-halo: rgb(170 155 199 / 14%);
  --popup-shadow: 0 10px 26px rgb(0 0 0 / 44%), 0 2px 8px rgb(0 0 0 / 26%);
}
[data-theme='dark'] .palette-sage.notification-drink {
  --popup-surface: linear-gradient(145deg, #17232c 0%, #121c24 100%);
  --popup-surface-solid: #162129;
  --popup-accent: #73b2d4;
  --popup-halo: rgb(115 178 212 / 14%);
  --popup-cta-bg: #26485c;
  --popup-cta-hover: #2f5870;
  --popup-cta-text: #d7effc;
}
[data-theme='dark'] .palette-sage.notification-rest {
  --popup-surface: linear-gradient(145deg, #172420 0%, #121c19 100%);
  --popup-surface-solid: #16221e;
  --popup-accent: #6fba97;
  --popup-halo: rgb(111 186 151 / 14%);
  --popup-cta-bg: #244b3c;
  --popup-cta-hover: #2d5c4a;
  --popup-cta-text: #d5f3e4;
}
[data-theme='dark'] .palette-sage.notification-eye_care {
  --popup-surface: linear-gradient(145deg, #25201a 0%, #1c1814 100%);
  --popup-surface-solid: #231e18;
  --popup-accent: #dca15e;
  --popup-halo: rgb(220 161 94 / 14%);
  --popup-cta-bg: #513c24;
  --popup-cta-hover: #63492c;
  --popup-cta-text: #fae5c8;
}

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
.appearance-minimal { border-inline-start: 3.5px solid var(--popup-accent); border-radius: 14px; }
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
