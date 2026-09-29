<script setup lang="ts">
import type { Language } from '../i18n'
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from '../i18n'

const props = defineProps<{ id: string }>()
const model = defineModel<Language>({ required: true })
const { languageOptions, t } = useI18n()
const root = ref<HTMLElement>()
const opened = ref(false)
const activeIndex = ref(0)
const selectedLabel = computed(() => languageOptions.find(option => option.value === model.value)?.label)

async function revealActive() {
  await nextTick()
  document.getElementById(`${props.id}-option-${activeIndex.value}`)?.scrollIntoView({ block: 'nearest' })
}

function open() {
  opened.value = true
  activeIndex.value = Math.max(0, languageOptions.findIndex(option => option.value === model.value))
  void revealActive()
}

function choose(index: number) {
  model.value = languageOptions[index]!.value
  opened.value = false
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && opened.value) {
    event.preventDefault()
    event.stopPropagation()
    opened.value = false
  }
  else if (['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter', ' '].includes(event.key)) {
    event.preventDefault()
    if (!opened.value) {
      open()
      return
    }
    if (event.key === 'Enter' || event.key === ' ') {
      choose(activeIndex.value)
      return
    }
    if (event.key === 'Home')
      activeIndex.value = 0
    else if (event.key === 'End')
      activeIndex.value = languageOptions.length - 1
    else activeIndex.value = Math.max(0, Math.min(languageOptions.length - 1, activeIndex.value + (event.key === 'ArrowDown' ? 1 : -1)))
    void revealActive()
  }
}

function closeOutside(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node))
    opened.value = false
}

onMounted(() => document.addEventListener('pointerdown', closeOutside))
onBeforeUnmount(() => document.removeEventListener('pointerdown', closeOutside))
</script>

<template>
  <div ref="root" class="language-picker">
    <button
      :id="id" type="button" class="language-trigger" role="combobox"
      :aria-label="`${t('settings.language')}: ${selectedLabel}`"
      :aria-expanded="opened" :aria-controls="`${id}-options`" aria-haspopup="listbox"
      :aria-activedescendant="opened ? `${id}-option-${activeIndex}` : undefined"
      @click="opened ? opened = false : open()" @keydown="onKeydown" @blur="opened = false"
    >
      <bdi>{{ selectedLabel }}</bdi><span aria-hidden="true">⌄</span>
    </button>
    <ul v-show="opened" :id="`${id}-options`" class="language-options" role="listbox" :aria-label="t('settings.language')">
      <li
        v-for="(option, index) in languageOptions" :id="`${id}-option-${index}`" :key="option.value"
        role="option" :aria-selected="model === option.value" :class="{ active: index === activeIndex }"
        @pointerdown.prevent @click="choose(index)"
      >
        <bdi :lang="option.value">{{ option.label }}</bdi><span v-if="model === option.value" aria-hidden="true">✓</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.language-picker { position: relative; min-width: 0; }
.language-trigger { width: 100%; display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 7px 9px; border: 1px solid var(--settings-glass-border); border-radius: 8px; background: var(--settings-glass-control); color: var(--text-primary); font-size: 12px; line-height: 1.5; text-align: start; }
.language-trigger:focus-visible { outline: 2px solid var(--color-primary); outline-offset: 2px; }
.language-options { position: absolute; inset-inline: 0; top: calc(100% + 5px); z-index: 10; margin: 0; padding: 4px; list-style: none; max-height: min(240px, 40dvh); overflow-y: auto; overscroll-behavior: contain; scrollbar-gutter: stable; scrollbar-width: thin; border: 1px solid var(--settings-glass-border); border-radius: 10px; background: var(--bg-primary); box-shadow: var(--shadow-lg); }
.language-options li { display: flex; align-items: center; justify-content: space-between; gap: 6px; padding: 8px; border-radius: 6px; font-size: 12px; color: var(--text-primary); cursor: pointer; }
.language-options li.active, .language-options li:hover { background: var(--settings-glass-selected); }
.language-options li[aria-selected='true'] { color: var(--color-primary); }
</style>
