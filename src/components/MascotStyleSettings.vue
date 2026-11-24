<script setup lang="ts">
import type { FrontendSettings } from '../types/data'
import type { NotificationPreviewRequest } from '../types/notificationPreview'
import type { MascotGroup, MascotStyle, PreviewReminderType } from '../utils/mascotStyles'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { computed, nextTick, onUnmounted, shallowRef, useId, useTemplateRef, watch } from 'vue'
import { useI18n } from '../i18n'
import NotificationApp from '../NotificationApp.vue'
import { getMascotAsset, getMascotGroup, mascotGroupOptions, mascotStyleOptions } from '../utils/mascotStyles'
import { resolveNotificationPalette } from '../utils/notificationPalette'
import { customReminderIconOptions, getLocalizedReminderVisual } from '../utils/reminderVisuals'
import NotificationCard from './NotificationCard.vue'

const props = defineProps<{
  settings: FrontendSettings
  disabled?: boolean
}>()
const model = defineModel<MascotStyle>({ required: true })
const { locale, t, clock, unit } = useI18n()
const groupId = useId()
const activeGroup = shallowRef<MascotGroup>(getMascotGroup(model.value))
const selectedGroup = computed(() => getMascotGroup(model.value))
const previewType = shallowRef<PreviewReminderType>('drink')
const previewIcon = shallowRef('custom_star')
const previewBusy = shallowRef(false)
const previewError = shallowRef('')
const browserPreview = shallowRef<NotificationPreviewRequest | null>(null)
const dialog = useTemplateRef<HTMLDialogElement>('previewDialog')

const inlineBreakMode = shallowRef(false)
const inlineCompleted = shallowRef(false)
const inlineCollapsed = shallowRef(false)
const inlineRemainingSeconds = shallowRef(0)
let inlineTimer: ReturnType<typeof setInterval> | null = null

const resolvedPalette = computed(() => resolveNotificationPalette(props.settings.notificationPalette, model.value))
const inlineVisual = computed(() =>
  getLocalizedReminderVisual(
    previewType.value,
    locale.value,
    previewType.value === 'custom' ? previewIcon.value : undefined,
    model.value,
  ),
)
const inlineActionEnabled = computed(() => previewType.value === 'rest' || previewType.value === 'eye_care')
const inlineTotalSeconds = computed(() => (previewType.value === 'rest' ? 300 : previewType.value === 'eye_care' ? 20 : 0))
const inlineActionTitle = computed(() => {
  if (inlineCompleted.value)
    return t(previewType.value === 'rest' ? 'notificationStatus.breakCompleted' : 'notificationStatus.actionCompleted')
  if (inlineBreakMode.value && previewType.value === 'rest')
    return t('notificationStatus.breakInProgress')
  if (previewType.value === 'eye_care')
    return t('notification.startEyeCare')
  if (previewType.value === 'rest')
    return t('notification.startBreak')
  return t('ui.startAction')
})
const inlineActionMessage = computed(() => {
  if (inlineCompleted.value)
    return t(previewType.value === 'rest' ? 'notificationStatus.breakCompletedHint' : 'notificationStatus.actionCompletedHint')
  if (previewType.value === 'eye_care')
    return t('notification.eyeHint')
  if (previewType.value === 'rest')
    return t('notification.restHint')
  return inlineVisual.value.shortMessage
})
const inlineBreakCountdownLabel = computed(() => clock(inlineRemainingSeconds.value, true))
const inlineProgress = computed(() => {
  if (!inlineTotalSeconds.value)
    return 1
  return Math.max(0, Math.min(1, inlineRemainingSeconds.value / inlineTotalSeconds.value))
})
const inlinePostponeOptions = computed(() => {
  const source = props.settings.postponeOptions.length > 0 ? props.settings.postponeOptions : [5, 10, 15]
  return source.map(minutes => ({ label: unit(minutes, 'minute'), minutes }))
})
const inlineCanReset = computed(() => inlineBreakMode.value || inlineCompleted.value || inlineCollapsed.value)

function stopInlineTimer() {
  if (inlineTimer !== null) {
    clearInterval(inlineTimer)
    inlineTimer = null
  }
}

function resetInlinePreview() {
  stopInlineTimer()
  inlineBreakMode.value = false
  inlineCompleted.value = false
  inlineCollapsed.value = false
  inlineRemainingSeconds.value = inlineTotalSeconds.value
}

function handleInlineComplete() {
  if (inlineActionEnabled.value) {
    stopInlineTimer()
    inlineCompleted.value = false
    inlineCollapsed.value = false
    inlineBreakMode.value = true
    inlineRemainingSeconds.value = inlineTotalSeconds.value
    inlineTimer = setInterval(() => {
      if (inlineRemainingSeconds.value <= 1) {
        inlineRemainingSeconds.value = 0
        stopInlineTimer()
        inlineCompleted.value = true
        return
      }
      inlineRemainingSeconds.value -= 1
    }, 1000)
    return
  }
  stopInlineTimer()
  inlineBreakMode.value = false
  inlineCompleted.value = true
}

function handleInlineFinish() {
  stopInlineTimer()
  inlineCollapsed.value = false
  inlineCompleted.value = true
}

function handleInlineCollapse(value: boolean) {
  inlineCollapsed.value = value
}

watch(model, (value) => {
  activeGroup.value = getMascotGroup(value)
})

watch(previewType, () => {
  resetInlinePreview()
})

onUnmounted(() => {
  stopInlineTimer()
})

async function previewReminder() {
  if (previewBusy.value || props.disabled)
    return
  previewBusy.value = true
  previewError.value = ''
  const request: NotificationPreviewRequest = {
    mascotStyle: model.value,
    reminderType: previewType.value,
    icon: previewType.value === 'custom' ? previewIcon.value : undefined,
    settings: { ...props.settings, postponeOptions: [...props.settings.postponeOptions] },
  }
  try {
    if (isTauri()) {
      await invoke('preview_notification', { request })
    }
    else {
      browserPreview.value = request
      await nextTick()
      dialog.value?.showModal()
    }
  }
  catch (error) {
    previewError.value = t('settings.previewFailed')
    console.error('Failed to preview notification:', error)
  }
  finally {
    previewBusy.value = false
  }
}

function closePreview() {
  dialog.value?.close()
  browserPreview.value = null
}
</script>

<template>
  <section class="mascot-settings" :aria-labelledby="`${groupId}-title`">
    <h3 :id="`${groupId}-title`" class="mascot-heading">
      {{ t('ui.companionTitle') }}
    </h3>
    <p class="mascot-description">
      {{ t('ui.companionDescription') }}
    </p>
    <div class="mascot-group-tabs" role="tablist" :aria-label="t('ui.companionTitle')">
      <button
        v-for="group in mascotGroupOptions"
        :key="group.id"
        type="button"
        role="tab"
        class="mascot-group-tab"
        :class="{ 'mascot-group-tab-active': activeGroup === group.id }"
        :aria-selected="activeGroup === group.id"
        :disabled="disabled"
        @click="activeGroup = group.id"
      >
        <span v-if="selectedGroup === group.id" class="mascot-group-dot" aria-hidden="true" />
        <span>{{ t(group.labelKey) }}</span>
      </button>
    </div>
    <div class="mascot-options" role="radiogroup" :aria-labelledby="`${groupId}-title`">
      <label
        v-for="option in mascotStyleOptions"
        v-show="option.group === activeGroup"
        :key="option.id"
        class="mascot-option"
        :class="{ 'mascot-option-selected': model === option.id }"
      >
        <input v-model="model" :name="groupId" :value="option.id" :disabled="disabled" class="mascot-radio" type="radio">
        <span class="mascot-thumbnail">
          <img
            :src="getMascotAsset(option.id, previewType === 'custom' ? option.previewState : previewType)"
            alt=""
            class="mascot-image"
            width="112"
            height="112"
          >
        </span>
        <span class="mascot-name">{{ t(option.nameKey) }}</span>
      </label>
    </div>

    <div class="live-preview-box">
      <div class="mascot-preview-controls">
        <label class="mascot-preview-label" :for="`${groupId}-state`">{{ t('ui.livePreview') }}</label>
        <select :id="`${groupId}-state`" v-model="previewType" class="mascot-state-select" :disabled="disabled">
          <option value="drink">
            {{ t('reminderTypes.drink.defaultName') }}
          </option>
          <option value="rest">
            {{ t('reminderTypes.rest.defaultName') }}
          </option>
          <option value="eye_care">
            {{ t('reminderTypes.eyeCare.defaultName') }}
          </option>
          <option value="custom">
            {{ t('reminderTypes.custom.defaultName') }}
          </option>
        </select>
        <button
          v-if="inlineCanReset"
          class="mascot-reset-button"
          type="button"
          :disabled="disabled"
          @click="resetInlinePreview"
        >
          {{ t('ui.resetPreview') }}
        </button>
        <button class="mascot-preview-button" type="button" :disabled="disabled || previewBusy" @click="previewReminder">
          {{ t('ui.desktopPreview') }}
        </button>
      </div>
      <div v-if="previewType === 'custom'" class="preview-custom-icons" :aria-label="t('form.customIcon')">
        <button v-for="option in customReminderIconOptions" :key="option.key" type="button" :disabled="disabled" :aria-pressed="previewIcon === option.key" :aria-label="t(option.labelKey)" :title="t(option.labelKey)" @click="previewIcon = option.key">
          <img :src="option.asset" alt="" width="28" height="28">
        </button>
      </div>
      <div v-if="!browserPreview" class="live-preview-stage">
        <NotificationCard
          visible
          preview
          :visual="inlineVisual"
          :name="inlineVisual.defaultName"
          :message="inlineVisual.shortMessage"
          :break-mode="inlineBreakMode"
          :completed="inlineCompleted"
          :collapsed="inlineCollapsed"
          :action-enabled="inlineActionEnabled"
          :action-title="inlineActionTitle"
          :action-message="inlineActionMessage"
          :break-countdown-label="inlineBreakCountdownLabel"
          :remaining-progress="inlineProgress"
          :remaining-seconds="inlineRemainingSeconds"
          :countdown-running="inlineBreakMode && !inlineCompleted"
          pending-label=""
          :postpone-options="inlinePostponeOptions"
          :appearance="settings.notificationAppearance"
          :palette="resolvedPalette"
          @complete="handleInlineComplete"
          @finish="handleInlineFinish"
          @collapse="handleInlineCollapse"
          @skip="resetInlinePreview"
          @postpone="resetInlinePreview"
        />
      </div>
    </div>

    <p class="mascot-hint">
      {{ t('settings.previewHint') }}
    </p>
    <p v-if="previewError" class="mascot-error" role="alert">
      {{ previewError }}
    </p>
    <dialog ref="previewDialog" class="mascot-preview-dialog" :aria-label="t('settings.previewReminder')" @cancel.prevent="closePreview" @click.self="closePreview" @close="browserPreview = null">
      <div v-if="browserPreview" class="mascot-preview-frame">
        <NotificationApp preview :preview-request="browserPreview" @closed="closePreview" />
      </div>
    </dialog>
  </section>
</template>

<style scoped>
.mascot-settings {
  padding-top: 16px;
}

.mascot-heading {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.mascot-description, .mascot-hint {
  margin-top: 6px;
  font-size: 11px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.mascot-group-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 4px;
  margin-top: 12px;
  padding: 3px;
  border-radius: 11px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
}

.mascot-group-tab {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 6px 8px;
  border-radius: 8px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  transition: background-color 140ms ease, color 140ms ease, box-shadow 140ms ease;
}

.mascot-group-tab:hover {
  color: var(--text-primary);
}

.mascot-group-tab-active {
  background: var(--bg-primary);
  color: var(--text-primary);
  font-weight: 600;
  box-shadow: var(--shadow-sm);
}

.mascot-group-dot {
  width: 6px;
  height: 6px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--color-primary);
}

.mascot-options {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  margin-top: 10px;
}

.mascot-option {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 7px 7px 10px;
  border: 1px solid var(--border-color);
  border-radius: 14px;
  cursor: pointer;
  background: var(--bg-primary);
  transition: border-color 150ms ease, box-shadow 150ms ease;
}

.mascot-option:hover {
  border-color: var(--color-primary);
}

.mascot-option-selected {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 1px var(--color-primary);
}

.mascot-option:has(:focus-visible) {
  outline: 2px solid var(--color-primary);
  outline-offset: 3px;
}

.mascot-option:has(:disabled) {
  opacity: 0.6;
  cursor: wait;
}

.mascot-radio {
  position: absolute;
  top: 11px;
  inset-inline-end: 11px;
  width: 14px;
  height: 14px;
  accent-color: var(--color-primary);
}

.mascot-thumbnail {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  background: var(--bg-secondary);
  overflow: hidden;
}

.mascot-image {
  width: 100%;
  height: 82px;
  object-fit: contain;
  padding: 6px;
}

.mascot-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  text-align: center;
}

.live-preview-box {
  margin-top: 14px;
  padding: 10px;
  border: 1px solid var(--border-color);
  border-radius: 14px;
  background: var(--bg-secondary);
}

.mascot-preview-controls {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.mascot-preview-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.mascot-state-select {
  min-width: 0;
  flex: 1 1 96px;
  padding: 6px 8px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 12px;
}

.mascot-reset-button {
  padding: 6px 10px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  transition: color 140ms ease, border-color 140ms ease;
}

.mascot-reset-button:hover {
  color: var(--text-primary);
  border-color: var(--color-primary);
}

.mascot-preview-button {
  padding: 7px 11px;
  border-radius: 9px;
  background: var(--color-primary);
  color: var(--color-primary-contrast);
  font-size: 12px;
  font-weight: 600;
}

.mascot-preview-button:hover {
  background: var(--color-primary-hover);
}

.mascot-preview-button:disabled {
  opacity: 0.6;
  cursor: wait;
}

.mascot-preview-button:focus-visible, .mascot-reset-button:focus-visible, .mascot-state-select:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}

.live-preview-stage {
  margin-top: 8px;
  border-radius: 12px;
  background: var(--bg-tertiary);
  overflow: hidden;
}

.live-preview-stage :deep(.notification-wrapper) {
  --notification-padding: 8px;
  height: auto;
  animation: none;
}

.live-preview-stage :deep(.notification-shell) {
  height: auto;
}

.mascot-error {
  margin-top: 8px;
  color: var(--color-danger);
  font-size: 12px;
}

.mascot-preview-dialog {
  position: fixed;
  inset: 0;
  margin: auto;
  padding: 0;
  width: 400px;
  max-width: 100vw;
  border: 0;
  background: transparent;
  overflow: visible;
}

.mascot-preview-dialog::backdrop {
  background: rgb(15 23 42 / 35%);
}

.mascot-preview-frame {
  width: 100%;
}
.preview-custom-icons { display: flex; gap: 8px; margin-top: 10px; }
.preview-custom-icons button { display: grid; place-items: center; width: 42px; height: 42px; border-radius: 10px; border: 1px solid var(--border-color); background: var(--bg-primary); }
.preview-custom-icons button[aria-pressed='true'] { border-color: var(--color-primary); background: var(--color-primary-light); }

@media (prefers-reduced-motion: reduce) {
  .mascot-option, .mascot-group-tab, .mascot-reset-button {
    transition: none;
  }
}
</style>
