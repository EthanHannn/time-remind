<script setup lang="ts">
import type { NotificationAppearance } from '../utils/notificationAppearance'
import { useId } from 'vue'
import { useI18n } from '../i18n'

defineProps<{ disabled?: boolean }>()
const model = defineModel<NotificationAppearance>({ required: true })
const id = useId()
const { t } = useI18n()
</script>

<template>
  <fieldset class="appearance-settings" :disabled="disabled">
    <legend>{{ t('settings.notificationAppearance') }}</legend>
    <div class="appearance-options">
      <label v-for="appearance in ['soft', 'classic'] as const" :key="appearance" class="appearance-option" :class="{ selected: model === appearance }">
        <input v-model="model" :name="id" type="radio" :value="appearance">
        <span class="appearance-sample" :class="appearance" aria-hidden="true"><i /><i /><i /></span>
        <span>{{ t(appearance === 'soft' ? 'settings.appearanceSoft' : 'settings.appearanceClassic') }}</span>
      </label>
    </div>
  </fieldset>
</template>

<style scoped>
.appearance-settings { border: 0; padding: 20px 0 0; min-width: 0; }
legend { float: left; width: 100%; font-size: 14px; font-weight: 600; color: var(--text-primary); margin-bottom: 10px; }
.appearance-options { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; clear: both; }
.appearance-option { display: grid; grid-template-columns: 1fr auto; align-items: center; gap: 10px; padding: 12px; border: 1px solid var(--border-color); border-radius: 12px; cursor: pointer; font-size: 12px; color: var(--text-primary); }
.appearance-option.selected { border-color: var(--color-primary); }
.appearance-option:has(:focus-visible) { outline: 2px solid var(--color-primary); outline-offset: 3px; }
.appearance-option input { grid-column: 2; grid-row: 1; accent-color: var(--color-primary); }
.appearance-option > span:last-child { grid-column: 1 / -1; }
.appearance-sample { display: flex; gap: 4px; grid-column: 1; grid-row: 1; }
.appearance-sample i { display: block; width: 24px; height: 14px; border-radius: 4px; background: #e9edf1; }
.soft i:nth-child(1) { background: #d3e3f1; }
.soft i:nth-child(2) { background: #d7e3ce; }
.soft i:nth-child(3) { background: #cce2df; }
.classic i:nth-child(1) { background: linear-gradient(135deg, #2f9fd8, #4f8cff); }
.classic i:nth-child(2) { background: linear-gradient(135deg, #57c59a, #4f8cff); }
.classic i:nth-child(3) { background: linear-gradient(135deg, #f6b35b, #4f8cff); }
[data-theme='dark'] .soft i:nth-child(1) { background: #364e63; }
[data-theme='dark'] .soft i:nth-child(2) { background: #42533d; }
[data-theme='dark'] .soft i:nth-child(3) { background: #345550; }
.appearance-settings:disabled { opacity: 0.6; }
</style>
