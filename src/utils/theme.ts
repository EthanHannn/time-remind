export type ThemeMode = 'light' | 'dark' | 'system'

export function createThemeController(root: HTMLElement, media?: MediaQueryList) {
  let mode: ThemeMode = 'system'
  let listening = false
  let revision = 0

  function render() {
    const dark = mode === 'dark' || (mode === 'system' && media?.matches)
    if (dark)
      root.setAttribute('data-theme', 'dark')
    else
      root.removeAttribute('data-theme')
    root.style.colorScheme = dark ? 'dark' : 'light'
  }

  function applyTheme(value: unknown) {
    mode = value === 'light' || value === 'dark' ? value : 'system'
    revision++
    if (mode === 'system' && !listening && media) {
      media.addEventListener('change', render)
      listening = true
    }
    else if (mode !== 'system' && listening) {
      media?.removeEventListener('change', render)
      listening = false
    }
    render()
  }

  function dispose() {
    if (listening)
      media?.removeEventListener('change', render)
    listening = false
  }

  applyTheme('system')
  return { applyTheme, dispose, revision: () => revision }
}
