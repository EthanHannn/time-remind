<script setup lang="ts">
import type { FrontendSettings } from '../types/data'
import type { NotificationPreviewRequest } from '../types/notificationPreview'
import type { MascotStyle, PreviewReminderType } from '../utils/mascotStyles'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { nextTick, shallowRef, useId, useTemplateRef } from 'vue'
import { useI18n } from '../i18n'
import NotificationApp from '../NotificationApp.vue'
import { mascotStyleOptions } from '../utils/mascotStyles'

const props = defineProps<{
  settings: FrontendSettings
  disabled?: boolean
}>()
const model = defineModel<MascotStyle>({ required: true })
const { t } = useI18n()
const groupId = useId()
const previewType = shallowRef<PreviewReminderType>('drink')
const previewBusy = shallowRef(false)
const previewError = shallowRef('')
const browserPreview = shallowRef<NotificationPreviewRequest | null>(null)
const dialog = useTemplateRef<HTMLDialogElement>('previewDialog')

async function previewReminder() {
  if (previewBusy.value || props.disabled)
    return
  previewBusy.value = true
  previewError.value = ''
  const request: NotificationPreviewRequest = {
    mascotStyle: model.value,
    reminderType: previewType.value,
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
      {{ t('settings.mascotStyle') }}
    </h3>
    <p class="mascot-description">
      {{ t('settings.mascotStyleDescription') }}
    </p>
    <div class="mascot-options" role="radiogroup" :aria-labelledby="`${groupId}-title`">
      <label v-for="option in mascotStyleOptions" :key="option.id" class="mascot-option" :class="{ 'mascot-option-selected': model === option.id }">
        <input v-model="model" :name="groupId" :value="option.id" :disabled="disabled" class="mascot-radio" type="radio">
        <span class="mascot-thumbnail" :class="{ 'mascot-thumbnail-paper': option.id !== 'classic' }">
          <img :src="option.image" alt="" class="mascot-image" width="112" height="112">
        </span>
        <span class="mascot-name">{{ t(option.nameKey) }}</span>
        <span class="mascot-caption">{{ t(option.descriptionKey) }}</span>
      </label>
    </div>
    <div class="mascot-preview-controls">
      <label class="mascot-preview-label" :for="`${groupId}-state`">{{ t('settings.previewState') }}</label>
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
      </select>
      <button class="mascot-preview-button" type="button" :disabled="disabled || previewBusy" @click="previewReminder">
        {{ t('settings.previewReminder') }}
      </button>
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
  padding-top: 22px;
}

.mascot-heading {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-secondary);
}

.mascot-description, .mascot-hint {
  margin-top: 6px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.mascot-options {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  margin-top: 14px;
}

.mascot-option {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 5px;
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
  top: 13px;
  right: 13px;
  width: 14px;
  height: 14px;
  accent-color: var(--color-primary);
}

.mascot-thumbnail {
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 9px;
  background: var(--bg-secondary);
  overflow: hidden;
}

.mascot-thumbnail-paper {
  background: #f7f5f0;
}

.mascot-image {
  width: 100%;
  height: auto;
  aspect-ratio: 1;
  object-fit: contain;
}

.mascot-name {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-primary);
}

.mascot-caption {
  font-size: 11px;
  line-height: 1.4;
  color: var(--text-secondary);
}

.mascot-preview-controls {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 16px;
}

.mascot-preview-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.mascot-state-select {
  min-width: 0;
  flex: 1;
  padding: 8px;
  border: 1px solid var(--border-color);
  border-radius: 9px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 12px;
}

.mascot-preview-button {
  padding: 9px 12px;
  border-radius: 10px;
  background: var(--color-primary);
  color: white;
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

.mascot-preview-button:focus-visible, .mascot-state-select:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 3px;
}

.mascot-error {
  margin-top: 8px;
  color: var(--text-primary);
  font-size: 12px;
}

.mascot-preview-dialog {
  position: fixed;
  inset: 0;
  margin: auto;
  padding: 0;
  width: 360px;
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
  height: 224px;
}

@media (prefers-reduced-motion: reduce) {
  .mascot-option {
    transition: none;
  }
}
</style>
