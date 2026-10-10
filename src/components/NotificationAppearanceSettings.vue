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
      <label v-for="appearance in ['soft', 'minimal', 'illustrated'] as const" :key="appearance" class="appearance-option" :class="{ selected: model === appearance }">
        <input v-model="model" :name="id" type="radio" :value="appearance">
        <span class="appearance-sample" :class="appearance" aria-hidden="true"><i /><i /><i /></span>
        <span>{{ t(`ui.${appearance}`) }}</span>
      </label>
    </div>
  </fieldset>
</template>

<style scoped>
.appearance-settings { border: 0; padding: 0; min-width: 0; }
legend { float: left; width: 100%; font-size: 13px; font-weight: 600; color: var(--text-primary); margin-bottom: 10px; }
.appearance-options { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; clear: both; }
.appearance-option { display: grid; grid-template-columns: 1fr auto; align-items: center; gap: 10px; padding: 10px; border: 1px solid var(--border-color); border-radius: 12px; cursor: pointer; font-size: 12px; color: var(--text-primary); }
.appearance-option.selected { border-color: var(--color-primary); }
.appearance-option:has(:focus-visible) { outline: 2px solid var(--color-primary); outline-offset: 3px; }
.appearance-option input { grid-column: 2; grid-row: 1; accent-color: var(--color-primary); }
.appearance-option > span:last-child { grid-column: 1 / -1; }
.appearance-sample { display: flex; gap: 4px; grid-column: 1; grid-row: 1; }
.appearance-sample i { display: block; width: 18px; height: 14px; border-radius: 4px; background: #e9edf1; }
.soft i:nth-child(1) { background: #d3e3f1; }
.soft i:nth-child(2) { background: #d7e3ce; }
.soft i:nth-child(3) { background: #f3e8d9; }
.minimal { width: 100%; flex-direction: column; border-inline-start: 2px solid #8aabba; padding-inline-start: 4px; }
.minimal i { height: 3px; width: 80%; }
.illustrated { width: 100%; height: 24px; align-items: center; background: linear-gradient(135deg, #f0f5f9, #dce9f1); border-radius: 5px; padding: 4px; }
.illustrated i:first-child { width: 20px; height: 18px; background: #8aabba; border-radius: 6px; }
.illustrated i:not(:first-child) { width: 12px; height: 4px; background: #9aaeba; }
[data-theme='dark'] .soft i:nth-child(1) { background: #364e63; }
[data-theme='dark'] .soft i:nth-child(2) { background: #42533d; }
[data-theme='dark'] .soft i:nth-child(3) { background: #514535; }
[data-theme='dark'] .illustrated { background: #233541; }
.appearance-settings:disabled { opacity: 0.6; }
</style>
