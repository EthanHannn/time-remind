<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, shallowRef, useId, useTemplateRef, watch } from 'vue'
import { useI18n } from '../i18n'
import AppIcon from './AppIcon.vue'

const props = defineProps<{ options: { minutes: number, label: string }[], disabled?: boolean }>()
const emit = defineEmits<{ postpone: [minutes: number] }>()
const { t } = useI18n()
const root = useTemplateRef<HTMLElement>('root')
const trigger = useTemplateRef<HTMLButtonElement>('trigger')
const open = shallowRef(false)
const id = useId()
const preferred = computed(() => props.options.find(option => option.minutes === 5) || props.options[0])

function close(restoreFocus = false) {
  open.value = false
  if (restoreFocus)
    trigger.value?.focus()
}
async function toggle() {
  if (open.value) {
    close(true)
    return
  }
  open.value = true
  await nextTick()
  root.value?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus()
}
function choose(minutes: number) {
  if (props.disabled)
    return
  close(true)
  emit('postpone', minutes)
}
function keydown(event: KeyboardEvent) {
  if (!open.value)
    return
  if (event.key === 'Escape') {
    event.stopPropagation()
    event.preventDefault()
    close(true)
  }
  else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
    event.preventDefault()
    const items = [...root.value!.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')]
    const index = items.indexOf(document.activeElement as HTMLButtonElement)
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length
    items[next]?.focus()
  }
  else if (event.key === 'Tab') {
    close()
  }
}
function outside(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node))
    close()
}
watch(() => props.disabled, disabled => disabled && close())
onMounted(() => document.addEventListener('pointerdown', outside))
onUnmounted(() => document.removeEventListener('pointerdown', outside))
</script>

<template>
  <div v-if="preferred" ref="root" class="snooze-control" @keydown="keydown">
    <button class="snooze-default" :disabled="disabled" type="button" @click="choose(preferred.minutes)">
      {{ t('ui.snoozeFor', { minutes: preferred.minutes }) }}
    </button>
    <button v-if="options.length > 1" ref="trigger" class="snooze-trigger" :disabled="disabled" type="button" :aria-label="t('ui.snoozeOptions')" :title="t('ui.snoozeOptions')" :aria-expanded="open" :aria-controls="id" aria-haspopup="menu" @click="toggle" @keydown.down.prevent="!open && toggle()">
      <AppIcon name="chevron-down" />
    </button>
    <div v-if="open" :id="id" class="snooze-menu" role="menu" :aria-label="t('ui.snoozeOptions')">
      <button v-for="option in options" :key="option.minutes" role="menuitem" type="button" class="snooze-option" @click="choose(option.minutes)">
        {{ option.label }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.snooze-control { position: relative; display: flex; min-width: 0; border: 1px solid var(--border-color); border-radius: 9px; }
.snooze-default, .snooze-trigger { color: var(--text-secondary); min-height: 36px; font-size: 12px; border-radius: 8px; }
.snooze-default { padding: 6px 10px; overflow-wrap: anywhere; flex: 1; }
.snooze-trigger { display: grid; place-items: center; width: 30px; flex: 0 0 30px; border-inline-start: 1px solid var(--border-color); border-start-start-radius: 0; border-end-start-radius: 0; }
.snooze-trigger .app-icon { width: 14px; height: 14px; }
.snooze-default:hover, .snooze-trigger:hover, .snooze-option:hover { background: var(--bg-secondary); color: var(--text-primary); }
.snooze-menu { position: absolute; bottom: calc(100% + 6px); inset-inline-end: 0; z-index: 5; width: max-content; min-width: 120px; max-width: 240px; max-height: 136px; overflow-y: auto; padding: 4px; border: 1px solid var(--border-color); border-radius: 10px; background: var(--bg-primary); box-shadow: 0 6px 20px rgb(0 0 0 / 12%); }
.snooze-option { display: block; width: 100%; text-align: start; border-radius: 6px; padding: 8px 12px; font-size: 12px; color: var(--text-primary); }
</style>
