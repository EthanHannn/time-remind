<script setup lang="ts">
import type { Reminder, ReminderRuntime } from '../types/reminder'
import { computed } from 'vue'
import { useI18n } from '../i18n'
import { getLocalizedReminderVisual } from '../utils/reminderVisuals'
import AppIcon from './AppIcon.vue'
import CountdownProgress from './CountdownProgress.vue'

const props = defineProps<{
  reminder: Reminder
  remainingSeconds?: number
  globallyPaused?: boolean
  globallySuppressed?: boolean
  pending?: boolean
  runtime?: ReminderRuntime
}>()
const emit = defineEmits<{ toggle: [id: string], edit: [id: string] }>()
const { locale, t, unit, clock } = useI18n()
const visual = computed(() => getLocalizedReminderVisual(props.reminder.reminder_type, locale.value, props.reminder.icon))
const totalSeconds = computed(() => props.runtime?.state === 'acting' ? props.reminder.action_duration_seconds : props.reminder.interval_minutes * 60)
const progress = computed(() => props.remainingSeconds === undefined || !props.reminder.enabled || totalSeconds.value <= 0 ? 0 : Math.max(0, Math.min(100, (1 - props.remainingSeconds / totalSeconds.value) * 100)))
const countdownRunning = computed(() => props.reminder.enabled && !props.globallyPaused
  && (!props.runtime || ['scheduled', 'acting', 'dnd'].includes(props.runtime.state)))
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
  const duration = unit(seconds % 60 === 0 ? seconds / 60 : seconds, seconds % 60 === 0 ? 'minute' : 'second')
  return props.reminder.reminder_type === 'rest' ? t('reminder.restDuration', { duration }) : duration
})
</script>

<template>
  <article
    class="reminder-card"
    :class="[`reminder-type-${visual.type}`, { 'reminder-card-disabled': !reminder.enabled }]"
    :aria-label="reminder.name"
  >
    <div class="card-top">
      <div class="icon-badge">
        <img v-if="visual.iconAsset" :src="visual.iconAsset" alt="" class="icon-image" width="26" height="26">
        <span v-else class="icon-fallback" aria-hidden="true">{{ visual.iconText }}</span>
      </div>
      <div class="card-copy">
        <h2 class="card-title" :title="reminder.name">
          {{ reminder.name }}
        </h2>
        <p class="card-meta">
          {{ unit(reminder.interval_minutes, 'minute') }}<span v-if="durationLabel"> · {{ durationLabel }}</span>
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
      <span class="countdown-panel">
        <span class="countdown-value" dir="ltr">{{ reminder.enabled && remainingSeconds !== undefined ? clock(remainingSeconds) : '--:--' }}</span>
        <span class="countdown-caption">{{ t('notification.remaining') }}</span>
      </span>
      <span v-if="showNextTrigger" class="next-trigger">{{ t('reminder.next', { time: nextTriggerLabel }) }}</span>
      <span v-else-if="specialStatus" class="special-status">{{ specialStatus }}</span>
      <span v-else class="schedule-placeholder" aria-hidden="true">—</span>
    </div>
    <div class="progress-track" aria-hidden="true">
      <CountdownProgress class="progress-bar" :value="progress / 100" :target="1" :remaining-seconds="remainingSeconds || 0" :running="countdownRunning" />
    </div>
  </article>
</template>

<style scoped>
.reminder-card {
  --card-accent: var(--type-custom);
  --card-soft: var(--type-custom-soft);
  position: relative;
  margin-bottom: 10px;
  padding: 13px 15px;
  border-radius: 16px;
  border: 1px solid var(--border-color);
  background: var(--card-surface);
  -webkit-backdrop-filter: blur(12px);
  backdrop-filter: blur(12px);
  box-shadow: var(--shadow-sm);
  transition: border-color 150ms ease, box-shadow 150ms ease, opacity 150ms ease;
}
.reminder-card:hover {
  border-color: var(--border-color-hover);
  box-shadow: var(--shadow-md);
}
.reminder-type-drink { --card-accent: var(--type-drink); --card-soft: var(--type-drink-soft); }
.reminder-type-rest { --card-accent: var(--type-rest); --card-soft: var(--type-rest-soft); }
.reminder-type-eye_care { --card-accent: var(--type-eye-care); --card-soft: var(--type-eye-care-soft); }
.reminder-type-custom { --card-accent: var(--type-custom); --card-soft: var(--type-custom-soft); }

.reminder-card-disabled { opacity: 0.58; }
.card-top { display: flex; align-items: center; gap: 11px; }
.icon-badge {
  width: 38px;
  height: 38px;
  flex: 0 0 38px;
  border-radius: 11px;
  background: var(--card-soft);
  display: grid;
  place-items: center;
}
.icon-image { width: 24px; height: 24px; object-fit: contain; }
.icon-fallback { font-size: 15px; font-weight: 700; color: var(--card-accent); }
.card-copy { min-width: 0; flex: 1; }
.card-title { margin: 0; overflow: hidden; font-size: 14px; font-weight: 650; line-height: 1.4; color: var(--text-primary); white-space: nowrap; text-overflow: ellipsis; }
.card-meta { margin-top: 2px; font-size: 11px; color: var(--text-secondary); }
.card-actions { display: flex; align-items: center; gap: 2px; }
.edit-button { display: grid; place-items: center; width: 30px; height: 32px; border-radius: 8px; color: var(--text-secondary); transition: background-color 140ms ease, color 140ms ease; }
.edit-button .app-icon { width: 15px; height: 15px; }
.edit-button:hover { color: var(--text-primary); background: var(--bg-secondary); }
.toggle-button { min-width: 42px; min-height: 34px; display: grid; place-items: center; border-radius: 8px; }
.toggle-pill { width: 34px; height: 20px; position: relative; border-radius: 999px; background: var(--bg-tertiary); border: 1px solid var(--border-color); transition: background-color 150ms ease, border-color 150ms ease; }
.toggle-button-active .toggle-pill { background: var(--color-primary); border-color: transparent; }
.toggle-thumb { position: absolute; top: 2px; left: 2px; width: 14px; height: 14px; border-radius: 50%; background: var(--bg-primary); box-shadow: 0 1px 2px rgb(0 0 0 / 20%); transition: transform 150ms ease, background-color 150ms ease; }
.toggle-button-active .toggle-thumb { transform: translateX(14px); background: var(--switch-thumb); }

.schedule-row { display: flex; align-items: baseline; flex-wrap: wrap; justify-content: space-between; gap: 4px 12px; margin-top: 12px; }
.countdown-panel { display: inline-flex; align-items: baseline; gap: 6px; }
.countdown-value { font-size: 18px; line-height: 1.2; font-weight: 700; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; color: var(--text-primary); }
.countdown-caption { font-size: 11px; color: var(--text-secondary); }
.next-trigger { margin-inline-start: auto; font-size: 12px; font-weight: 500; font-variant-numeric: tabular-nums; color: var(--text-secondary); }
.special-status { margin-inline-start: auto; padding: 2px 8px; border-radius: 999px; background: var(--card-soft); font-size: 11px; font-weight: 500; color: var(--card-accent); }
.schedule-placeholder { margin-inline-start: auto; color: var(--text-tertiary); font-size: 12px; }

.progress-track { margin-top: 8px; height: 5px; border-radius: 999px; background: var(--bg-secondary); overflow: hidden; }
.progress-bar { height: 100%; transform-origin: left; background: var(--card-accent); border-radius: inherit; }
[dir='rtl'] .progress-bar { transform-origin: right; }
</style>
