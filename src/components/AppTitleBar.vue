<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { onMounted, onUnmounted, shallowRef } from 'vue'
import { useI18n } from '../i18n'
import { appIconMain } from '../utils/reminderVisuals'

const appWindow = getCurrentWindow()
const { t } = useI18n()
const isMaximized = shallowRef(false)
let unlistenResized: (() => void) | null = null

async function refreshMaximized() {
  try {
    isMaximized.value = await appWindow.isMaximized()
  }
  catch {
    isMaximized.value = false
  }
}

async function startDragging(event: MouseEvent) {
  if (event.button !== 0 || event.detail > 1)
    return

  try {
    await appWindow.startDragging()
  }
  catch {
    // 忽略拖拽启动失败
  }
}

async function minimizeWindow() {
  try {
    await appWindow.minimize()
  }
  catch {
    // 忽略最小化失败
  }
}

async function toggleMaximizeWindow() {
  try {
    await appWindow.toggleMaximize()
    await refreshMaximized()
  }
  catch {
    await refreshMaximized()
  }
}

async function hideWindow() {
  try {
    await appWindow.hide()
  }
  catch {
    // 忽略隐藏窗口失败
  }
}

onMounted(async () => {
  await refreshMaximized()
  unlistenResized = await appWindow.onResized(() => {
    void refreshMaximized()
  })
})

onUnmounted(() => {
  unlistenResized?.()
})
</script>

<template>
  <div class="app-titlebar">
    <div
      class="app-titlebar-drag-region"
      @mousedown="startDragging"
      @dblclick="toggleMaximizeWindow"
    >
      <div class="app-titlebar-brand">
        <img :src="appIconMain" :alt="t('common.appName')" class="app-titlebar-logo" draggable="false">
        <span class="app-titlebar-name">{{ t('common.appName') }}</span>
      </div>
    </div>

    <div class="app-titlebar-controls">
      <button
        class="app-titlebar-button"
        type="button"
        :aria-label="t('windowControls.minimize')"
        @click="minimizeWindow"
      >
        <span class="app-titlebar-icon app-titlebar-icon-minimize" />
      </button>
      <button
        class="app-titlebar-button"
        type="button"
        :aria-label="isMaximized ? t('windowControls.restore') : t('windowControls.maximize')"
        @click="toggleMaximizeWindow"
      >
        <span
          class="app-titlebar-icon"
          :class="isMaximized ? 'app-titlebar-icon-restore' : 'app-titlebar-icon-maximize'"
        />
      </button>
      <button
        class="app-titlebar-button app-titlebar-button-close"
        type="button"
        :aria-label="t('windowControls.hideToTray')"
        @click="hideWindow"
      >
        <span class="app-titlebar-icon app-titlebar-icon-close" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.app-titlebar {
  height: 44px;
  flex: 0 0 44px;
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid rgba(148, 163, 184, 0.14);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.62), rgba(255, 255, 255, 0.26)),
    linear-gradient(90deg, rgba(47, 159, 216, 0.08), rgba(246, 179, 91, 0.06));
  user-select: none;
}

[data-theme='dark'] .app-titlebar {
  border-bottom-color: rgba(148, 163, 184, 0.12);
  background:
    linear-gradient(180deg, rgba(20, 24, 31, 0.82), rgba(16, 20, 28, 0.42)),
    linear-gradient(90deg, rgba(47, 159, 216, 0.14), rgba(246, 179, 91, 0.08));
}

.app-titlebar-drag-region {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  padding-left: 14px;
}

.app-titlebar-brand {
  display: inline-flex;
  align-items: center;
  gap: 9px;
  min-width: 0;
}

.app-titlebar-logo {
  width: 22px;
  height: 22px;
  flex-shrink: 0;
  object-fit: contain;
}

.app-titlebar-name {
  min-width: 0;
  overflow: hidden;
  color: var(--text-primary);
  font-size: 13px;
  font-weight: 700;
  line-height: 1;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-titlebar-controls {
  position: relative;
  z-index: 80;
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 7px 8px 7px 4px;
}

.app-titlebar-button {
  width: 34px;
  height: 30px;
  border-radius: 10px;
  color: var(--text-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition:
    background-color 0.18s ease,
    color 0.18s ease,
    transform 0.18s ease;
}

.app-titlebar-button:hover {
  background: rgba(255, 255, 255, 0.76);
  color: var(--text-primary);
}

.app-titlebar-button:active {
  transform: scale(0.94);
}

[data-theme='dark'] .app-titlebar-button:hover {
  background: rgba(255, 255, 255, 0.08);
}

.app-titlebar-button-close:hover {
  background: #e35d6a;
  color: white;
}

[data-theme='dark'] .app-titlebar-button-close:hover {
  background: #e35d6a;
  color: white;
}

.app-titlebar-icon {
  position: relative;
  width: 13px;
  height: 13px;
  display: block;
}

.app-titlebar-icon-minimize::before {
  position: absolute;
  left: 1px;
  right: 1px;
  bottom: 3px;
  height: 1.5px;
  border-radius: 999px;
  background: currentcolor;
  content: '';
}

.app-titlebar-icon-maximize::before {
  position: absolute;
  inset: 1px;
  border: 1.5px solid currentcolor;
  border-radius: 2px;
  content: '';
}

.app-titlebar-icon-restore::before,
.app-titlebar-icon-restore::after {
  position: absolute;
  width: 8px;
  height: 8px;
  border: 1.5px solid currentcolor;
  border-radius: 2px;
  content: '';
}

.app-titlebar-icon-restore::before {
  top: 1px;
  right: 1px;
}

.app-titlebar-icon-restore::after {
  left: 1px;
  bottom: 1px;
  background: inherit;
}

.app-titlebar-icon-close::before,
.app-titlebar-icon-close::after {
  position: absolute;
  top: 6px;
  left: 1px;
  width: 11px;
  height: 1.5px;
  border-radius: 999px;
  background: currentcolor;
  content: '';
}

.app-titlebar-icon-close::before {
  transform: rotate(45deg);
}

.app-titlebar-icon-close::after {
  transform: rotate(-45deg);
}
</style>
