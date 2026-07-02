<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'

const props = withDefaults(defineProps<{
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
  loading?: boolean
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

function handleKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape')
    return

  event.preventDefault()
  handleCancel()
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div class="confirm-overlay" @click.self="handleCancel">
    <div class="confirm-container">
      <div class="confirm-header">
        <h2 class="confirm-title">
          {{ title }}
        </h2>
      </div>

      <p class="confirm-message">
        {{ message }}
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
  border-radius: 28px;
  border: 1px solid rgba(148, 163, 184, 0.16);
  background:
    radial-gradient(circle at top right, rgba(227, 93, 106, 0.12), transparent 30%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(255, 255, 255, 0.92));
  box-shadow: 0 30px 60px rgba(15, 23, 42, 0.22);
  padding: 24px;
}

[data-theme='dark'] .confirm-container {
  background:
    radial-gradient(circle at top right, rgba(227, 93, 106, 0.16), transparent 30%),
    linear-gradient(180deg, rgba(24, 28, 37, 0.96), rgba(16, 20, 28, 0.94));
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
  border-color: rgba(148, 163, 184, 0.16);
  background: rgba(255, 255, 255, 0.72);
  color: var(--text-primary);
}

[data-theme='dark'] .confirm-button-secondary {
  background: rgba(24, 29, 38, 0.84);
}

.confirm-button-secondary:hover:not(:disabled) {
  box-shadow: 0 12px 24px rgba(15, 23, 42, 0.08);
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
