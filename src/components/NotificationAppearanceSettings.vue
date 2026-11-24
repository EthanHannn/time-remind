<script setup lang="ts">
import type { NotificationAppearance } from '../utils/notificationAppearance'
import type { NotificationPaletteSetting } from '../utils/notificationPalette'
import { useId } from 'vue'
import { useI18n } from '../i18n'
import { notificationPaletteOptions } from '../utils/notificationPalette'

defineProps<{ disabled?: boolean }>()
const model = defineModel<NotificationAppearance>({ required: true })
const palette = defineModel<NotificationPaletteSetting>('palette', { default: 'auto' })
const id = useId()
const paletteId = useId()
const { t } = useI18n()

const paletteLabelKeys: Record<NotificationPaletteSetting, string> = {
  auto: 'ui.paletteAuto',
  paper: 'ui.palettePaper',
  ink: 'ui.paletteInk',
  sage: 'ui.paletteSage',
}
</script>

<template>
  <div class="appearance-stack">
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

    <fieldset class="palette-settings" :disabled="disabled">
      <legend>{{ t('ui.notificationPalette') }}</legend>
      <div class="palette-options">
        <label v-for="option in notificationPaletteOptions" :key="option" class="palette-option" :class="{ selected: palette === option }">
          <input v-model="palette" :name="paletteId" type="radio" :value="option">
          <span class="palette-swatch" :class="`swatch-${option}`" aria-hidden="true"><i /><i /></span>
          <span class="palette-label">{{ t(paletteLabelKeys[option]) }}</span>
        </label>
      </div>
    </fieldset>
  </div>
</template>

<style scoped>
.appearance-stack { display: flex; flex-direction: column; gap: 14px; }
.appearance-settings, .palette-settings { border: 0; padding: 0; min-width: 0; }
legend { float: left; width: 100%; font-size: 13px; font-weight: 600; color: var(--text-primary); margin-bottom: 10px; }
.appearance-options { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; clear: both; }
.appearance-option { display: grid; grid-template-columns: 1fr auto; align-items: center; gap: 10px; padding: 10px; border: 1px solid var(--border-color); border-radius: 12px; cursor: pointer; font-size: 12px; color: var(--text-primary); transition: border-color 140ms ease, background-color 140ms ease; }
.appearance-option:hover, .palette-option:hover { border-color: var(--color-primary); }
.appearance-option.selected, .palette-option.selected { border-color: var(--color-primary); box-shadow: 0 0 0 1px var(--color-primary); }
.appearance-option:has(:focus-visible), .palette-option:has(:focus-visible) { outline: 2px solid var(--color-primary); outline-offset: 3px; }
.appearance-option input { grid-column: 2; grid-row: 1; accent-color: var(--color-primary); }
.appearance-option > span:last-child { grid-column: 1 / -1; }
.appearance-sample { display: flex; gap: 4px; grid-column: 1; grid-row: 1; }
.appearance-sample i { display: block; width: 18px; height: 14px; border-radius: 4px; background: #e9edf1; }
.soft i:nth-child(1) { background: #dbeafe; }
.soft i:nth-child(2) { background: #d1fae5; }
.soft i:nth-child(3) { background: #fee2d5; }
.minimal { width: 100%; flex-direction: column; border-inline-start: 2px solid var(--color-primary); padding-inline-start: 4px; }
.minimal i { height: 3px; width: 80%; }
.illustrated { width: 100%; height: 24px; align-items: center; background: linear-gradient(135deg, #fff8f4, #f9ece4); border-radius: 5px; padding: 4px; }
.illustrated i:first-child { width: 20px; height: 18px; background: #e56747; border-radius: 6px; }
.illustrated i:not(:first-child) { width: 12px; height: 4px; background: #a8968e; }
[data-theme='dark'] .soft i:nth-child(1) { background: #263e5c; }
[data-theme='dark'] .soft i:nth-child(2) { background: #1e4238; }
[data-theme='dark'] .soft i:nth-child(3) { background: #4d2e24; }
[data-theme='dark'] .illustrated { background: #261e1d; }

.palette-options { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; clear: both; }
.palette-option { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border: 1px solid var(--border-color); border-radius: 11px; cursor: pointer; font-size: 12px; color: var(--text-primary); background: var(--bg-primary); transition: border-color 140ms ease, box-shadow 140ms ease; }
.palette-option input { position: absolute; opacity: 0; pointer-events: none; }
.palette-swatch { display: inline-flex; width: 22px; height: 16px; border-radius: 5px; overflow: hidden; border: 1px solid rgb(0 0 0 / 12%); flex-shrink: 0; }
.palette-swatch i { flex: 1; display: block; }
.swatch-auto i:first-child { background: #e56747; }
.swatch-auto i:last-child { background: #8b85ff; }
.swatch-paper i:first-child { background: #fcf9f7; }
.swatch-paper i:last-child { background: #e56747; }
.swatch-ink i:first-child { background: #171924; }
.swatch-ink i:last-child { background: #8b85ff; }
.swatch-sage i:first-child { background: #f6f8fc; }
.swatch-sage i:last-child { background: #3b66f5; }
.palette-label { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
.appearance-settings:disabled, .palette-settings:disabled { opacity: 0.6; }
</style>
