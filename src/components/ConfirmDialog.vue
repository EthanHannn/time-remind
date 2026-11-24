<script setup lang="ts">
import { useTemplateRef } from 'vue'
import { useModalFocus } from '../composables/useModalFocus'

const props = withDefaults(defineProps<{
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
  loading?: boolean
  error?: string
}>(), {
  confirmText: 'Confirm',
  cancelText: 'Cancel',
  danger: false,
  loading: false,
})

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()

function handleCancel() {
  if (props.loading)
    return

  emit('cancel')
}

function handleConfirm() {
  if (props.loading)
    return

  emit('confirm')
}

useModalFocus(useTemplateRef<HTMLElement>('dialog'), handleCancel)
</script>

<template>
  <div class="confirm-overlay" @click.self="handleCancel">
    <div ref="dialog" class="confirm-container" role="dialog" aria-modal="true" :aria-label="title" tabindex="-1">
      <div class="confirm-header">
        <h2 class="confirm-title">
          {{ title }}
        </h2>
      </div>

      <p class="confirm-message">
        {{ message }}
      </p>

      <p v-if="error" role="alert">
        {{ error }}
      </p>

      <div class="confirm-actions">
        <button
          class="confirm-button confirm-button-secondary"
          :disabled="loading"
          type="button"
          @click="handleCancel"
        >
          {{ cancelText }}
        </button>

        <button
          class="confirm-button"
          :class="{ 'confirm-button-danger': danger }"
          :disabled="loading"
          type="button"
          @click="handleConfirm"
        >
          {{ confirmText }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  z-index: 110;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgba(7, 10, 16, 0.48);
  backdrop-filter: blur(8px);
}

.confirm-container {
  width: min(100%, 420px);
  border-radius: 20px;
  border: 1px solid var(--border-color);
  background: var(--app-shell-bg);
  box-shadow: var(--shadow-md);
  padding: 22px;
}

.confirm-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.confirm-title {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
}

.confirm-message {
  margin: 14px 0 0;
  font-size: 14px;
  line-height: 1.7;
  color: var(--text-secondary);
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 24px;
}

.confirm-button {
  min-width: 92px;
  padding: 11px 16px;
  border-radius: 16px;
  border: 1px solid transparent;
  background: linear-gradient(135deg, #d95767, #f07c88);
  color: white;
  font-size: 14px;
  font-weight: 600;
  transition:
    transform 0.18s ease,
    box-shadow 0.18s ease,
    opacity 0.18s ease;
}

.confirm-button:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 16px 28px rgba(217, 87, 103, 0.22);
}

.confirm-button:active:not(:disabled) {
  transform: scale(0.97);
}

.confirm-button-secondary {
  border-color: var(--border-color);
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.confirm-button-secondary:hover:not(:disabled) {
  background: var(--bg-tertiary);
}

.confirm-button-danger {
  background: linear-gradient(135deg, #d95767, #f07c88);
}

.confirm-button:disabled {
  opacity: 0.58;
  cursor: not-allowed;
}

@media (max-width: 520px) {
  .confirm-actions {
    flex-direction: column-reverse;
  }

  .confirm-button {
    width: 100%;
  }
}
</style>
