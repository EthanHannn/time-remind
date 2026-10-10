// @vitest-environment jsdom
import { expect, it } from 'vitest'
import { createThemeController } from './theme'

it('tracks system changes, preserves an explicit choice, and cleans up the listener', () => {
  const root = document.createElement('div')
  const callbacks = new Set<() => void>()
  const media = {
    matches: false,
    addEventListener: (_: string, callback: () => void) => callbacks.add(callback),
    removeEventListener: (_: string, callback: () => void) => callbacks.delete(callback),
  }
  const controller = createThemeController(root, media as unknown as MediaQueryList)
  expect(callbacks.size).toBe(1)
  media.matches = true
  callbacks.forEach(callback => callback())
  expect(root.dataset.theme).toBe('dark')
  expect(root.style.colorScheme).toBe('dark')
  controller.applyTheme('light')
  expect(callbacks.size).toBe(0)
  expect(root.dataset.theme).toBeUndefined()
  expect(root.style.colorScheme).toBe('light')
  controller.applyTheme('system')
  controller.applyTheme('system')
  expect(callbacks.size).toBe(1)
  expect(root.dataset.theme).toBe('dark')
  controller.dispose()
  expect(callbacks.size).toBe(0)
})
