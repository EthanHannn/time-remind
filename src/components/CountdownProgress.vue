<script setup lang="ts">
import { computed, onMounted, onUnmounted, shallowRef, useTemplateRef, watch } from 'vue'

const props = defineProps<{
  value: number
  target: 0 | 1
  remainingSeconds: number
  running: boolean
}>()
const fill = useTemplateRef<HTMLDivElement>('fill')
const value = computed(() => Math.max(0, Math.min(1, props.value)))
const reducedMotion = shallowRef(false)
let animation: Animation | undefined
let motionPreference: MediaQueryList | undefined

function updateMotionPreference() {
  reducedMotion.value = motionPreference?.matches ?? false
}

onMounted(() => {
  motionPreference = window.matchMedia?.('(prefers-reduced-motion: reduce)')
  updateMotionPreference()
  motionPreference?.addEventListener('change', updateMotionPreference)
})

watch([fill, value, () => props.target, () => props.remainingSeconds, () => props.running, reducedMotion], ([element, nextValue, target, remaining, running, reduce], previous) => {
  if (!element)
    return

  // A new countdown resets immediately. Regular clock updates continue from
  // the currently painted position, avoiding jumps when IPC delivery jitters.
  const reset = previous[0] !== element || previous[2] !== target
    || (nextValue - previous[1]) * (target === 1 ? 1 : -1) < 0
  if (!reduce && remaining > 0 && !running && animation && !reset) {
    animation.pause()
    return
  }
  // Keep both keyframes as scaleX: a matrix-to-zero-scale interpolation can
  // become discrete because the destination matrix cannot be decomposed.
  const from = animation && !reset ? new DOMMatrixReadOnly(getComputedStyle(element).transform).a : nextValue
  animation?.cancel()
  animation = undefined
  if (!running || reduce || remaining <= 0 || !element.animate)
    return
  animation = element.animate([
    { transform: `scaleX(${from})` },
    { transform: `scaleX(${target})` },
  ], { duration: remaining * 1000, easing: 'linear', fill: 'forwards' })
}, { flush: 'post' })

onUnmounted(() => {
  animation?.cancel()
  motionPreference?.removeEventListener('change', updateMotionPreference)
})
</script>

<template>
  <div ref="fill" :style="{ transform: `scaleX(${value})` }" />
</template>
