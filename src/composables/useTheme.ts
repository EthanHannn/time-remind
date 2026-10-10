import { onUnmounted } from 'vue'
import { createThemeController } from '../utils/theme'

// One controller per WebView, shared by the main window and its settings dialog.
let controller: ReturnType<typeof createThemeController> | undefined
let consumers = 0

export function useTheme() {
  controller ??= createThemeController(document.documentElement, window.matchMedia?.('(prefers-color-scheme: dark)'))
  const current = controller
  consumers++
  onUnmounted(() => {
    if (--consumers === 0) {
      current.dispose()
      controller = undefined
    }
  })
  return { applyTheme: current.applyTheme, themeRevision: current.revision }
}
