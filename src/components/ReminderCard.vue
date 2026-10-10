<script setup lang="ts">
import type { Reminder, ReminderRuntime } from '../types/reminder'
import { computed } from 'vue'
import { useI18n } from '../i18n'
import { getLocalizedReminderVisual } from '../utils/reminderVisuals'
import AppIcon from './AppIcon.vue'

const props = defineProps<{
  reminder: Reminder
  remainingSeconds?: number
  globallyPaused?: boolean
  globallySuppressed?: boolean
  pending?: boolean
  runtime?: ReminderRuntime
}>()
const emit = defineEmits<{ toggle: [id: string], edit: [id: string] }>()
const { locale, t } = useI18n()
const visual = computed(() => getLocalizedReminderVisual(props.reminder.reminder_type, locale.value, props.reminder.icon))
const totalSeconds = computed(() => props.runtime?.state === 'acting' ? props.reminder.action_duration_seconds : props.reminder.interval_minutes * 60)
const progress = computed(() => props.remainingSeconds === undefined || !props.reminder.enabled || totalSeconds.value <= 0 ? 0 : Math.max(0, Math.min(100, (1 - props.remainingSeconds / totalSeconds.value) * 100)))
const cardStyle = computed(() => ({ '--visual-accent': visual.value.accent, '--visual-soft': visual.value.accentSoft, '--visual-border': visual.value.borderSoft }))
const specialStatus = computed(() => {
  if (!props.reminder.enabled || props.globallyPaused || props.globallySuppressed)
    return ''
  const state = props.runtime?.state
  if (state === 'paused')
    return t(props.runtime?.reason === 'system' ? 'runtime.systemPaused' : 'reminder.paused')
  if (state === 'dnd')
    return t(props.runtime?.reason === 'fullscreen' ? 'settings.fullscreenDelay' : 'settings.dnd')
  return state && !['scheduled', 'disabled'].includes(state) ? t(`runtime.${state}`) : ''
})
const showNextTrigger = computed(() => props.reminder.enabled && !props.globallyPaused && !props.globallySuppressed && (!props.runtime || props.runtime.state === 'scheduled'))
const nextTriggerLabel = computed(() => {
  if (!props.reminder.next_trigger)
    return t('common.noSchedule')
  const raw = props.reminder.next_trigger
  const date = new Date(/Z$|[+-]\d{2}:\d{2}$/.test(raw) ? raw : `${raw}Z`)
  if (Number.isNaN(date.getTime()))
    return t('common.noSchedule')
  const sameDay = date.toDateString() === new Date().toDateString()
  return new Intl.DateTimeFormat(locale.value, { ...(sameDay ? {} : { month: 'numeric' as const, day: 'numeric' as const }), hour: '2-digit', minute: '2-digit' }).format(date)
})
const durationLabel = computed(() => {
  if (!props.reminder.action_enabled || props.reminder.action_duration_seconds <= 0)
    return ''
  const seconds = props.reminder.action_duration_seconds
  if (props.reminder.reminder_type === 'rest' && seconds % 60 === 0)
    return t('reminder.restDuration', { minutes: seconds / 60 })
  return `${seconds % 60 === 0 ? seconds / 60 : seconds} ${t(seconds % 60 === 0 ? 'common.minutes' : 'common.seconds')}`
})
function formatTime(seconds: number) {
  const safe = Math.max(0, seconds)
  const h = Math.floor(safe / 3600)
  const m = Math.floor(safe % 3600 / 60)
  const s = String(safe % 60).padStart(2, '0')
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${s}` : `${m}:${s}`
}
</script>

<template>
  <article class="reminder-card" :class="{ 'reminder-card-disabled': !reminder.enabled }" :style="cardStyle" :aria-label="reminder.name">
    <div class="card-top">
      <img v-if="visual.iconAsset" :src="visual.iconAsset" alt="" class="icon-image" width="32" height="32">
      <span v-else aria-hidden="true">{{ visual.iconText }}</span>
      <div class="card-copy">
        <h2 class="card-title" :title="reminder.name">
          {{ reminder.name }}
        </h2>
        <p class="card-meta">
          {{ reminder.interval_minutes }} {{ t('common.minutes') }}<span v-if="durationLabel"> · {{ durationLabel }}</span>
        </p>
      </div>
      <div class="card-actions">
        <button class="edit-button" :disabled="pending" type="button" :aria-label="`${t('ui.edit')}: ${reminder.name}`" :title="t('ui.edit')" @click="emit('edit', reminder.id)">
          <AppIcon name="edit" />
        </button>
        <button class="toggle-button" :disabled="pending" :class="{ 'toggle-button-active': reminder.enabled }" role="switch" :aria-checked="reminder.enabled" :aria-label="reminder.name" :title="t(reminder.enabled ? 'reminder.enabled' : 'runtime.disabled')" type="button" @click="emit('toggle', reminder.id)">
          <span class="toggle-pill"><span class="toggle-thumb" /></span>
        </button>
      </div>
    </div>
    <div class="schedule-row">
      <span v-if="showNextTrigger" class="next-trigger">{{ t('reminder.next', { time: nextTriggerLabel }) }}</span>
      <span v-else-if="specialStatus" class="special-status">{{ specialStatus }}</span>
      <span v-else class="schedule-placeholder" aria-hidden="true">—</span>
      <span class="countdown-panel">
        {{ t('notification.remaining') }} <span class="countdown-value">{{ reminder.enabled && remainingSeconds !== undefined ? formatTime(remainingSeconds) : '--:--' }}</span>
      </span>
    </div>
    <div class="progress-track" aria-hidden="true">
      <div class="progress-bar" :style="{ transform: `scaleX(${progress / 100})` }" />
    </div>
  </article>
</template>

<style scoped>
.reminder-card {
  margin-bottom: 10px; padding: 13px 14px; border-radius: 13px; border: 1px solid var(--visual-border);
  background: rgba(255, 255, 255, 0.72);
  -webkit-backdrop-filter: blur(12px);
  backdrop-filter: blur(12px);
  box-shadow: 0 3px 12px rgba(15, 23, 42, 0.04), inset 0 1px 0 rgba(255, 255, 255, 0.66);
}
[data-theme='dark'] .reminder-card {
  background: rgba(24, 29, 38, 0.72);
  box-shadow: 0 3px 12px rgba(2, 6, 23, 0.16), inset 0 1px 0 rgba(255, 255, 255, 0.05);
}
.reminder-card-disabled { opacity: 0.65; }
.card-top { display: flex; align-items: center; gap: 10px; }
.icon-image { flex: 0 0 32px; object-fit: contain; }
.card-copy { min-width: 0; flex: 1; }
.card-title { margin: 0; overflow: hidden; font-size: 14px; font-weight: 650; line-height: 1.5; color: var(--text-primary); white-space: nowrap; text-overflow: ellipsis; }
.card-meta { margin-top: 2px; font-size: 11px; color: var(--text-secondary); }
.card-actions { display: flex; align-items: center; gap: 4px; }
.edit-button { display: grid; place-items: center; width: 30px; height: 32px; border-radius: 8px; color: var(--text-secondary); }
.edit-button .app-icon { width: 16px; height: 16px; }
.edit-button:hover { color: var(--color-primary); background: var(--bg-secondary); }
.toggle-button { min-width: 44px; min-height: 36px; display: grid; place-items: center; border-radius: 8px; }
.toggle-pill { width: 34px; height: 20px; position: relative; border-radius: 999px; background: #8a97a8; transition: background-color 150ms ease; }
.toggle-button-active .toggle-pill { background: var(--color-primary); }
.toggle-thumb { position: absolute; top: 3px; left: 3px; width: 14px; height: 14px; border-radius: 50%; background: white; transition: transform 150ms ease; }
.toggle-button-active .toggle-thumb { transform: translateX(14px); }
.schedule-row { display: flex; align-items: baseline; flex-wrap: wrap; justify-content: space-between; gap: 4px 12px; margin-top: 12px; }
.next-trigger { font-size: 16px; line-height: 1.5; font-weight: 650; font-variant-numeric: tabular-nums; color: var(--text-primary); }
.special-status { font-size: 12px; color: var(--text-secondary); }
.schedule-placeholder { color: var(--text-tertiary); }
.countdown-panel { margin-inline-start: auto; font-size: 11px; color: var(--text-secondary); }
.countdown-value { font-variant-numeric: tabular-nums; }
.progress-track { margin-top: 7px; height: 4px; border-radius: 4px; background: var(--border-color); overflow: hidden; }
.progress-bar { height: 100%; transform-origin: left; background: linear-gradient(90deg, var(--visual-accent), #6f9fdf); border-radius: inherit; transition: transform 250ms linear; }
[dir='rtl'] .progress-bar { transform-origin: right; }
</style>
