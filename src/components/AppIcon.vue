<script setup lang="ts">
defineProps<{ name: 'plus' | 'close' | 'edit' | 'trash' | 'settings' | 'stats' | 'pause' | 'play' | 'chevron-down' | 'expand' | 'collapse' }>()
const paths = {
  'plus': 'M12 5v14M5 12h14',
  'close': 'm6 6 12 12M6 18 18 6',
  'edit': 'm15 5 4 4M4 20l5-1L20 8a2.8 2.8 0 0 0-4-4L5 15l-1 5Z',
  'trash': 'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v5M14 11v5',
  'settings': 'm9 3-.6 2.4-2.2 1L4 5.8 2 9l1.8 1.7v2.6L2 15l2 3.2 2.2-.6 2.2 1L9 21h6l.6-2.4 2.2-1 2.2.6 2-3.2-1.8-1.7v-2.6L22 9l-2-3.2-2.2.6-2.2-1L15 3H9ZM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
  'stats': 'M4 3v17h17M9 15v-5M14 15V6M19 15v-8',
  'pause': 'M8 5v14M16 5v14',
  'play': 'm8 4 12 8-12 8V4Z',
  'chevron-down': 'm6 9 6 6 6-6',
  'expand': 'M8 3H3v5M16 3h5v5M3 16v5h5M21 16v5h-5',
  'collapse': 'M3 8h5V3M21 8h-5V3M8 21v-5H3M16 21v-5h5',
}
</script>

<template>
  <svg class="app-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
    <path :d="paths[name]" />
  </svg>
</template>

<style scoped>
.app-icon { display: block; flex: 0 0 auto; }
</style>
