<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

type ResizeDirection = 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West'

const appWindow = getCurrentWindow()
const handles: Array<{ direction: ResizeDirection, className: string }> = [
  { direction: 'North', className: 'resize-handle resize-handle-top' },
  { direction: 'South', className: 'resize-handle resize-handle-bottom' },
  { direction: 'West', className: 'resize-handle resize-handle-left' },
  { direction: 'East', className: 'resize-handle resize-handle-right' },
  { direction: 'NorthWest', className: 'resize-handle resize-handle-corner resize-handle-top-left' },
  { direction: 'NorthEast', className: 'resize-handle resize-handle-corner resize-handle-top-right' },
  { direction: 'SouthWest', className: 'resize-handle resize-handle-corner resize-handle-bottom-left' },
  { direction: 'SouthEast', className: 'resize-handle resize-handle-corner resize-handle-bottom-right' },
]

async function startResize(direction: ResizeDirection, event: MouseEvent) {
  if (event.button !== 0)
    return

  try {
    await appWindow.startResizeDragging(direction)
  }
  catch {
    // 忽略缩放启动失败
  }
}
</script>

<template>
  <div class="resize-handles" aria-hidden="true">
    <div
      v-for="handle in handles"
      :key="handle.direction"
      :class="handle.className"
      @mousedown="startResize(handle.direction, $event)"
    />
  </div>
</template>

<style scoped>
.resize-handles {
  position: absolute;
  inset: 0;
  z-index: 60;
  pointer-events: none;
}

.resize-handle {
  position: absolute;
  pointer-events: auto;
}

.resize-handle-top {
  top: 0;
  left: 12px;
  right: 12px;
  height: 4px;
  cursor: ns-resize;
}

.resize-handle-bottom {
  right: 12px;
  bottom: 0;
  left: 12px;
  height: 6px;
  cursor: ns-resize;
}

.resize-handle-left {
  top: 12px;
  bottom: 12px;
  left: 0;
  width: 6px;
  cursor: ew-resize;
}

.resize-handle-right {
  top: 12px;
  right: 0;
  bottom: 12px;
  width: 6px;
  cursor: ew-resize;
}

.resize-handle-corner {
  width: 12px;
  height: 12px;
}

.resize-handle-top-left {
  top: 0;
  left: 0;
  cursor: nwse-resize;
}

.resize-handle-top-right {
  top: 0;
  right: 0;
  cursor: nesw-resize;
}

.resize-handle-bottom-left {
  bottom: 0;
  left: 0;
  cursor: nesw-resize;
}

.resize-handle-bottom-right {
  right: 0;
  bottom: 0;
  cursor: nwse-resize;
}
</style>
