// @vitest-environment jsdom
import { invoke } from '@tauri-apps/api/core'
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import SettingsPage from './SettingsPage.vue'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-autostart', () => ({ isEnabled: vi.fn().mockResolvedValue(false), enable: vi.fn(), disable: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ confirm: vi.fn(), open: vi.fn(), save: vi.fn() }))

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
  localStorage.clear()
  vi.stubGlobal('matchMedia', vi.fn(() => ({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() })))
  vi.mocked(invoke).mockReset().mockImplementation(async (command) => {
    if (command === 'get_all_settings')
      return { language: 'zh-CN', sound_volume: '0', dnd_enabled: 'true', dnd_start: '23:00', dnd_end: '07:00' }
    if (command === 'get_platform_capabilities')
      return { supportsAutostart: true, supportsSilentStart: true, supportsFullscreenDetection: true }
    return undefined
  })
})
afterEach(() => {
  vi.useRealTimers()
})

it('does not save while loading and reports a failed batch with a working retry', async () => {
  const wrapper = mount(SettingsPage, { global: { stubs: { LanguageSelect: true, MascotStyleSettings: true, NotificationAppearanceSettings: true } } })
  await flushPromises()
  await vi.advanceTimersByTimeAsync(400)
  expect(vi.mocked(invoke).mock.calls.filter(([name]) => name === 'save_settings')).toHaveLength(0)
  vi.mocked(invoke).mockRejectedValueOnce('disk full')
  await wrapper.findAll('.theme-button')[1]!.trigger('click')
  await vi.advanceTimersByTimeAsync(400)
  await flushPromises()
  expect(wrapper.find('[role="alert"]').text()).toContain('disk full')
  vi.mocked(invoke).mockResolvedValueOnce(undefined)
  await wrapper.find('[role="alert"] button').trigger('click')
  await flushPromises()
  expect(wrapper.find('[role="alert"]').exists()).toBe(false)
  const saves = vi.mocked(invoke).mock.calls.filter(([name]) => name === 'save_settings')
  expect(saves).toHaveLength(2)
  expect(saves[1]![1]).toMatchObject({ settings: { theme: 'dark', sound_volume: '0' } })
  wrapper.unmount()
})
