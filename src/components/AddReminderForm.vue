<script setup lang="ts">
import type { CreateReminderRequest } from '../types/reminder'
import { computed, ref, watch } from 'vue'
import { useI18n } from '../i18n'
import { getLocalizedReminderTypeOptions, getLocalizedReminderVisual } from '../utils/reminderVisuals'

const emit = defineEmits<{
  submit: [data: CreateReminderRequest]
  cancel: []
}>()

const name = ref('')
const message = ref('')
const intervalMinutes = ref(60)
const actionEnabled = ref(false)
const actionTitle = ref('')
const actionMessage = ref('')
const actionDurationValue = ref(0)
const actionDurationUnit = ref<'seconds' | 'minutes'>('seconds')
const selectedType = ref('drink')
const errorMessage = ref('')
const { locale, t } = useI18n()

const reminderTypeOptions = computed(() => getLocalizedReminderTypeOptions(locale.value))
const selectedVisual = computed(() => getLocalizedReminderVisual(selectedType.value, locale.value))
const isRestType = computed(() => selectedType.value === 'rest')
const actionDurationSeconds = computed(() =>
  actionDurationUnit.value === 'minutes'
    ? actionDurationValue.value * 60
    : actionDurationValue.value,
)
const actionDurationUnitLabel = computed(() => t('common.durationUnit'))
const secondsLabel = computed(() => t('common.seconds'))

function applyTypeDefaults(type: string) {
  const visual = getLocalizedReminderVisual(type, locale.value)
  name.value = visual.defaultName
  message.value = visual.shortMessage
  intervalMinutes.value = visual.defaultIntervalMinutes
  actionEnabled.value = Boolean(visual.defaultActionDurationSeconds)
  actionDurationUnit.value = type === 'eye_care' ? 'seconds' : 'minutes'
  actionDurationValue.value = actionDurationUnit.value === 'minutes'
    ? Math.max(1, Math.round((visual.defaultActionDurationSeconds ?? 0) / 60))
    : (visual.defaultActionDurationSeconds ?? 20)
  actionTitle.value = type === 'rest'
    ? t('notification.startBreak')
    : visual.label
  actionMessage.value = visual.shortMessage
}

watch([selectedType, locale], () => applyTypeDefaults(selectedType.value), { immediate: true })

function getIcon(): string {
  return selectedType.value
}

function handleSubmit() {
  errorMessage.value = ''

  if (!name.value.trim() || !message.value.trim()) {
    errorMessage.value = t('form.requiredError')
    return
  }

  if (!Number.isInteger(intervalMinutes.value) || intervalMinutes.value < 1 || intervalMinutes.value > 1440) {
    errorMessage.value = t('form.intervalError')
    return
  }

  if (actionEnabled.value && (!Number.isInteger(actionDurationSeconds.value) || actionDurationSeconds.value < 1 || actionDurationSeconds.value > 7200)) {
    errorMessage.value = t('form.actionDurationError')
    return
  }

  if (actionEnabled.value && (!actionTitle.value.trim() || !actionMessage.value.trim())) {
    errorMessage.value = t('form.requiredError')
    return
  }

  const normalizedBreakDuration = isRestType.value && actionEnabled.value
    ? Math.max(1, Math.ceil(actionDurationSeconds.value / 60))
    : 0

  emit('submit', {
    name: name.value.trim(),
    reminder_type: selectedType.value,
    icon: getIcon(),
    message: message.value.trim(),
    interval_minutes: intervalMinutes.value,
    break_duration_minutes: normalizedBreakDuration,
    break_notification_enabled: isRestType.value && actionEnabled.value,
    action_enabled: actionEnabled.value,
    action_title: actionEnabled.value ? actionTitle.value.trim() : '',
    action_message: actionEnabled.value ? actionMessage.value.trim() : '',
    action_duration_seconds: actionEnabled.value ? actionDurationSeconds.value : 0,
    action_completion_mode: 'auto',
  })
}
</script>

<template>
  <div class="form-overlay" @click.self="emit('cancel')">
    <div class="form-container">
      <div class="form-header">
        <div>
          <h3 class="form-title">
            {{ t('form.title') }}
          </h3>
          <p class="form-subtitle">
            {{ t('form.subtitle') }}
          </p>
        </div>

        <button class="close-button" type="button" @click="emit('cancel')">
          <span class="close-symbol">×</span>
        </button>
      </div>

      <div class="form-group">
        <label class="group-label">{{ t('form.type') }}</label>
        <div class="type-grid">
          <button
            v-for="type in reminderTypeOptions"
            :key="type.type"
            class="type-card"
            :class="{ 'type-card-active': selectedType === type.type }"
            :style="{
              '--type-accent': type.accent,
              '--type-soft': type.accentSoft,
              '--type-border': type.borderSoft,
            }"
            type="button"
            @click="selectedType = type.type"
          >
            <div class="type-visual">
              <img
                v-if="type.iconAsset"
                :src="type.iconAsset"
                :alt="type.label"
                class="type-image"
              >
              <span v-else class="type-fallback-icon">{{ type.iconText }}</span>
            </div>

            <div class="type-copy">
              <span class="type-label">{{ type.label }}</span>
              <span class="type-description">{{ type.description }}</span>
            </div>
          </button>
        </div>
      </div>

      <div class="form-group">
        <label class="group-label">{{ t('form.name') }}</label>
        <input
          v-model="name"
          class="text-input"
          type="text"
          :placeholder="`${t('common.example')}：${selectedVisual.defaultName}`"
        >
      </div>

      <div class="form-group">
        <label class="group-label">{{ t('form.content') }}</label>
        <input
          v-model="message"
          class="text-input"
          type="text"
          :placeholder="selectedVisual.shortMessage"
        >
      </div>

      <div class="inline-grid">
        <div class="form-group">
          <label class="group-label">{{ t('form.interval') }}</label>
          <input
            v-model.number="intervalMinutes"
            class="text-input"
            type="number"
            min="1"
            max="1440"
          >
        </div>
      </div>

      <div class="rest-options">
        <div class="rest-tip-card">
          <div>
            <div class="rest-tip-title">
              {{ t('form.restCountdown') }}
            </div>
            <div class="rest-tip-copy">
              {{ t('form.restCountdownDescription') }}
            </div>
          </div>

          <label class="switch">
            <input v-model="actionEnabled" type="checkbox">
            <span class="slider" />
          </label>
        </div>
      </div>

      <div v-if="actionEnabled" class="action-options">
        <div class="inline-grid">
          <div class="form-group">
            <label class="group-label">{{ t('form.actionDuration') }}</label>
            <input
              v-model.number="actionDurationValue"
              class="text-input"
              type="number"
              min="1"
              max="7200"
            >
          </div>

          <div class="form-group">
            <label class="group-label">{{ actionDurationUnitLabel }}</label>
            <select v-model="actionDurationUnit" class="text-input">
              <option value="seconds">
                {{ secondsLabel }}
              </option>
              <option value="minutes">
                {{ t('common.minutes') }}
              </option>
            </select>
          </div>
        </div>

        <div class="inline-grid">
          <div class="form-group">
            <label class="group-label">{{ t('form.name') }}</label>
            <input v-model="actionTitle" class="text-input" type="text">
          </div>

          <div class="form-group">
            <label class="group-label">{{ t('form.content') }}</label>
            <input v-model="actionMessage" class="text-input" type="text">
          </div>
        </div>
      </div>

      <p v-if="errorMessage" class="error-message">
        {{ errorMessage }}
      </p>

      <div class="form-actions">
        <button class="btn-secondary" type="button" @click="emit('cancel')">
          {{ t('common.cancel') }}
        </button>
        <button class="btn-primary" type="button" @click="handleSubmit">
          {{ t('form.submit') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.form-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgba(7, 10, 16, 0.48);
  backdrop-filter: blur(8px);
}

.form-container {
  width: min(100%, 560px);
  max-height: min(88vh, 760px);
  overflow-y: auto;
  border-radius: 28px;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background:
    radial-gradient(circle at top right, rgba(47, 159, 216, 0.12), transparent 28%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(255, 255, 255, 0.9));
  box-shadow: 0 30px 60px rgba(15, 23, 42, 0.22);
  padding: 24px;
}

[data-theme='dark'] .form-container {
  background:
    radial-gradient(circle at top right, rgba(47, 159, 216, 0.16), transparent 28%),
    linear-gradient(180deg, rgba(24, 28, 37, 0.96), rgba(16, 20, 28, 0.94));
}

.form-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 22px;
}

.form-title {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
}

.form-subtitle {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.close-button {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  background: rgba(148, 163, 184, 0.12);
  transition: transform 0.2s ease, background-color 0.2s ease;
}

.close-button:hover {
  background: rgba(148, 163, 184, 0.18);
}

.close-button:active {
  transform: scale(0.94);
}

.close-symbol {
  font-size: 20px;
  line-height: 1;
}

.form-group {
  margin-bottom: 18px;
}

.group-label {
  display: block;
  margin-bottom: 10px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
}

.type-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.type-card {
  padding: 12px;
  display: flex;
  align-items: center;
  gap: 12px;
  border-radius: 18px;
  border: 1px solid rgba(148, 163, 184, 0.16);
  background: rgba(255, 255, 255, 0.72);
  text-align: left;
  transition:
    transform 0.18s ease,
    border-color 0.18s ease,
    box-shadow 0.18s ease,
    background-color 0.18s ease;
}

[data-theme='dark'] .type-card {
  background: rgba(22, 27, 36, 0.82);
}

.type-card:hover {
  transform: translateY(-1px);
  border-color: var(--type-border);
  box-shadow: 0 12px 24px rgba(15, 23, 42, 0.08);
}

.type-card-active {
  border-color: var(--type-border);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.92), var(--type-soft));
  box-shadow: 0 14px 24px rgba(15, 23, 42, 0.08);
}

[data-theme='dark'] .type-card-active {
  background: linear-gradient(180deg, rgba(25, 31, 43, 0.96), rgba(25, 31, 43, 0.9));
}

.type-visual {
  width: 52px;
  height: 52px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 16px;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.6), var(--type-soft));
}

.type-image {
  width: 32px;
  height: 32px;
  object-fit: contain;
}

.type-fallback-icon {
  font-size: 16px;
  font-weight: 700;
  color: var(--type-accent);
}

.type-copy {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.type-label {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
}

.type-description {
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.inline-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}

.text-input {
  width: 100%;
  padding: 12px 14px;
  border-radius: 16px;
  border: 1px solid rgba(148, 163, 184, 0.22);
  background: rgba(255, 255, 255, 0.78);
  color: var(--text-primary);
  outline: none;
  transition:
    border-color 0.18s ease,
    box-shadow 0.18s ease,
    background-color 0.18s ease;
}

[data-theme='dark'] .text-input {
  background: rgba(20, 24, 31, 0.88);
}

.text-input:focus {
  border-color: rgba(47, 159, 216, 0.4);
  box-shadow: 0 0 0 4px rgba(47, 159, 216, 0.12);
}

.rest-options {
  margin-bottom: 18px;
}

.rest-tip-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  border-radius: 18px;
  border: 1px solid rgba(87, 197, 154, 0.22);
  background: rgba(87, 197, 154, 0.08);
}

.rest-tip-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
}

.rest-tip-copy {
  margin-top: 4px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.switch {
  position: relative;
  display: inline-block;
  width: 48px;
  height: 26px;
  flex-shrink: 0;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  inset: 0;
  border-radius: 999px;
  background-color: rgba(148, 163, 184, 0.36);
  transition: background-color 0.2s ease;
}

.slider::before {
  content: '';
  position: absolute;
  left: 3px;
  bottom: 3px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: white;
  box-shadow: 0 2px 6px rgba(15, 23, 42, 0.22);
  transition: transform 0.22s ease;
}

input:checked + .slider {
  background-color: #57C59A;
}

input:checked + .slider::before {
  transform: translateX(22px);
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 26px;
}

.btn-secondary,
.btn-primary {
  padding: 11px 18px;
  border-radius: 999px;
  font-size: 14px;
  font-weight: 600;
  transition: transform 0.2s ease, box-shadow 0.2s ease, background-color 0.2s ease;
}

.btn-secondary {
  background: rgba(148, 163, 184, 0.14);
  color: var(--text-primary);
}

.btn-primary {
  background: linear-gradient(135deg, #2f9fd8, #4f8cff);
  color: white;
  box-shadow: 0 14px 24px rgba(47, 159, 216, 0.24);
}

.btn-secondary:active,
.btn-primary:active {
  transform: scale(0.96);
}

.error-message {
  margin: 0;
  padding: 10px 12px;
  border-radius: 14px;
  background: rgba(227, 93, 106, 0.12);
  color: #d95767;
  font-size: 13px;
}

@media (max-width: 520px) {
  .type-grid,
  .inline-grid {
    grid-template-columns: 1fr;
  }

  .form-actions {
    flex-direction: column-reverse;
  }

  .btn-secondary,
  .btn-primary {
    width: 100%;
  }
}
</style>
