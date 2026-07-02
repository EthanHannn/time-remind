<script setup lang="ts">
import type { Reminder } from '../types/reminder'
import { computed } from 'vue'
import { useI18n } from '../i18n'
import { getLocalizedReminderVisual } from '../utils/reminderVisuals'

const props = defineProps<{
  reminder: Reminder
  remainingSeconds?: number
}>()

const emit = defineEmits<{
  toggle: [id: string]
  delete: [id: string]
}>()

const { locale, t } = useI18n()
const visual = computed(() => getLocalizedReminderVisual(props.reminder.reminder_type, locale.value))

const totalSeconds = computed(() => props.reminder.interval_minutes * 60)
const progress = computed(() => {
  if (!props.remainingSeconds || !props.reminder.enabled) {
    return 0
  }

  return Math.max(0, Math.min(100, (1 - props.remainingSeconds / totalSeconds.value) * 100))
})

const cardStyle = computed(() => ({
  '--visual-accent': visual.value.accent,
  '--visual-soft': visual.value.accentSoft,
  '--visual-border': visual.value.borderSoft,
  '--visual-badge': visual.value.badgeBackground,
}))

const statusLabel = computed(() => props.reminder.enabled ? t('reminder.enabled') : t('reminder.paused'))
const restBreakLabel = computed(() => {
  if (props.reminder.reminder_type !== 'rest' || props.reminder.break_duration_minutes <= 0) {
    return ''
  }

  return t('reminder.restDuration', { minutes: props.reminder.break_duration_minutes })
})

const restBreakNotificationLabel = computed(() => {
  if (props.reminder.reminder_type !== 'rest') {
    return ''
  }

  return props.reminder.break_notification_enabled ? t('reminder.restTipOn') : t('reminder.restTipOff')
})

const nextTriggerLabel = computed(() => {
  if (!props.reminder.next_trigger) {
    return t('common.noSchedule')
  }

  const nextTrigger = new Date(`${props.reminder.next_trigger}Z`)
  if (Number.isNaN(nextTrigger.getTime())) {
    return t('common.noSchedule')
  }

  return nextTrigger.toLocaleTimeString(locale.value, {
    hour: '2-digit',
    minute: '2-digit',
  })
})

function formatTime(seconds: number): string {
  const safeSeconds = Math.max(0, seconds)
  const h = Math.floor(safeSeconds / 3600)
  const m = Math.floor((safeSeconds % 3600) / 60)
  const s = safeSeconds % 60

  if (h > 0) {
    return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
  }

  return `${m}:${String(s).padStart(2, '0')}`
}
</script>

<template>
  <article
    class="reminder-card"
    :class="{ 'reminder-card-disabled': !reminder.enabled }"
    :style="cardStyle"
  >
    <div class="card-top">
      <div class="card-main">
        <div class="icon-badge">
          <img
            v-if="visual.iconAsset"
            :src="visual.iconAsset"
            :alt="visual.label"
            class="icon-image"
          >
          <span v-else class="fallback-icon-text">{{ visual.iconText }}</span>
        </div>

        <div class="card-copy">
          <div class="title-row">
            <h3 class="card-title">
              {{ reminder.name }}
            </h3>
            <span class="status-chip">
              {{ statusLabel }}
            </span>
          </div>

          <p class="card-message">
            {{ reminder.message }}
          </p>

          <div class="meta-row">
            <span class="meta-item">
              {{ reminder.interval_minutes }} {{ t('common.minutes') }}
            </span>
            <span v-if="restBreakLabel" class="meta-item">
              {{ restBreakLabel }}
            </span>
            <span v-if="restBreakNotificationLabel" class="meta-item">
              {{ restBreakNotificationLabel }}
            </span>
            <span class="meta-item">
              {{ t('reminder.next', { time: nextTriggerLabel }) }}
            </span>
          </div>
        </div>
      </div>

      <div class="countdown-panel">
        <span class="countdown-label">{{ t('reminder.countdown') }}</span>
        <span class="countdown-value">
          {{ reminder.enabled && remainingSeconds !== undefined ? formatTime(remainingSeconds) : '--:--' }}
        </span>
      </div>
    </div>

    <div class="progress-track">
      <div class="progress-bar" :style="{ width: `${progress}%` }" />
    </div>

    <div class="card-actions">
      <button class="action-link danger" type="button" @click="emit('delete', reminder.id)">
        {{ t('common.delete') }}
      </button>

      <button
        class="toggle-button"
        :class="{ 'toggle-button-active': reminder.enabled }"
        type="button"
        @click="emit('toggle', reminder.id)"
      >
        <span class="toggle-label">
          {{ reminder.enabled ? t('reminder.running') : t('reminder.paused') }}
        </span>
        <span class="toggle-pill">
          <span class="toggle-thumb" />
        </span>
      </button>
    </div>
  </article>
</template>

<style scoped>
.reminder-card {
  position: relative;
  margin-bottom: 14px;
  padding: 16px;
  border-radius: 24px;
  border: 1px solid var(--visual-border);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.9), rgba(255, 255, 255, 0.76)),
    var(--visual-soft);
  box-shadow:
    0 18px 30px rgba(15, 23, 42, 0.08),
    inset 0 1px 0 rgba(255, 255, 255, 0.66);
  transition:
    transform 0.22s ease,
    box-shadow 0.22s ease,
    border-color 0.22s ease,
    opacity 0.22s ease;
}

.reminder-card:hover {
  transform: translateY(-2px);
  box-shadow:
    0 24px 40px rgba(15, 23, 42, 0.1),
    inset 0 1px 0 rgba(255, 255, 255, 0.7);
}

[data-theme='dark'] .reminder-card {
  background:
    linear-gradient(180deg, rgba(23, 28, 37, 0.92), rgba(18, 22, 31, 0.9)),
    var(--visual-soft);
  box-shadow:
    0 18px 30px rgba(2, 6, 23, 0.3),
    inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.reminder-card-disabled {
  opacity: 0.72;
}

.reminder-card-disabled:hover {
  transform: none;
}

.card-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.card-main {
  flex: 1;
  display: flex;
  align-items: flex-start;
  gap: 14px;
  min-width: 0;
}

.icon-badge {
  width: 56px;
  height: 56px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 18px;
  background: var(--visual-badge);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.48);
}

.icon-image {
  width: 36px;
  height: 36px;
  object-fit: contain;
}

.fallback-icon-text {
  font-size: 16px;
  font-weight: 700;
  color: var(--visual-accent);
}

.card-copy {
  min-width: 0;
  flex: 1;
}

.title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  justify-content: space-between;
}

.card-title {
  margin: 0;
  min-width: 0;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.status-chip {
  flex-shrink: 0;
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  color: var(--visual-accent);
  background: var(--visual-soft);
}

.card-message {
  margin: 8px 0 0;
  font-size: 13px;
  line-height: 1.55;
  color: var(--text-secondary);
}

.meta-row {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 10px;
}

.meta-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}

.countdown-panel {
  min-width: 84px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
}

.countdown-label {
  font-size: 11px;
  color: var(--text-tertiary);
}

.countdown-value {
  font-size: 18px;
  font-weight: 700;
  color: var(--visual-accent);
  letter-spacing: -0.03em;
  font-variant-numeric: tabular-nums;
}

.progress-track {
  height: 6px;
  margin-top: 14px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.18);
}

.progress-bar {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--visual-accent), rgba(255, 255, 255, 0.92));
  transition: width 0.35s ease;
}

.card-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 14px;
}

.action-link {
  padding: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  transition: color 0.2s ease;
}

.action-link:hover {
  color: var(--text-primary);
}

.action-link.danger:hover {
  color: #e35d6a;
}

.toggle-button {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background: rgba(255, 255, 255, 0.7);
  transition:
    transform 0.2s ease,
    border-color 0.2s ease,
    background-color 0.2s ease;
}

[data-theme='dark'] .toggle-button {
  background: rgba(28, 33, 44, 0.78);
}

.toggle-button:active {
  transform: scale(0.97);
}

.toggle-button-active {
  border-color: var(--visual-border);
}

.toggle-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.toggle-pill {
  width: 34px;
  height: 20px;
  position: relative;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.26);
  transition: background-color 0.2s ease;
}

.toggle-button-active .toggle-pill {
  background: var(--visual-accent);
}

.toggle-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: white;
  box-shadow: 0 2px 6px rgba(15, 23, 42, 0.2);
  transition: transform 0.22s ease;
}

.toggle-button-active .toggle-thumb {
  transform: translateX(14px);
}

@media (max-width: 420px) {
  .card-top {
    flex-direction: column;
  }

  .countdown-panel {
    width: 100%;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }
}
</style>
