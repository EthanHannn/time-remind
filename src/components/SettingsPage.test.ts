// @vitest-environment jsdom
import { invoke } from '@tauri-apps/api/core'
import { confirm, open } from '@tauri-apps/plugin-dialog'
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { useI18n } from '../i18n'
import SettingsPage from './SettingsPage.vue'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-autostart', () => ({ isEnabled: vi.fn().mockResolvedValue(false), enable: vi.fn(), disable: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ confirm: vi.fn(), open: vi.fn(), save: vi.fn() }))

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
  localStorage.clear()
  vi.mocked(confirm).mockReset()
  vi.mocked(open).mockReset()
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
  expect(wrapper.find('[role="alert"]').text()).toContain(useI18n().t('operations.unexpected'))
  vi.mocked(invoke).mockResolvedValueOnce(undefined)
  await wrapper.find('[role="alert"] button').trigger('click')
  await flushPromises()
  expect(wrapper.find('[role="alert"]').exists()).toBe(false)
  const saves = vi.mocked(invoke).mock.calls.filter(([name]) => name === 'save_settings')
  expect(saves).toHaveLength(2)
  expect(saves[1]![1]).toMatchObject({ settings: { theme: 'dark', sound_volume: '0' } })
  wrapper.unmount()
})

it.each([false, true])('renders structured import feedback with an explicit warning state (%s)', async (warning) => {
  vi.mocked(confirm).mockResolvedValue(true)
  vi.mocked(open).mockResolvedValue('backup.json')
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === 'get_all_settings')
      return { language: 'en-US' }
    if (command === 'get_platform_capabilities')
      return { supportsAutostart: true, supportsSilentStart: true, supportsFullscreenDetection: true }
    if (command === 'read_file')
      return '{}'
    if (command === 'import_data')
      return { mode: 'replace', reminders_imported: 1, logs_imported: 2, settings_imported: 3, skipped_logs: 4, backup_path: 'before-import.json', autostart_sync_failed: warning }
  })
  const wrapper = mount(SettingsPage, { global: { stubs: { LanguageSelect: true, MascotStyleSettings: true, NotificationAppearanceSettings: true } } })
  await flushPromises()
  await wrapper.findAll('.data-actions .data-button')[1]!.trigger('click')
  await flushPromises()
  expect(confirm).toHaveBeenCalled()
  expect(invoke).toHaveBeenCalledWith('import_data', { data: {}, mode: 'replace' })
  const message = wrapper.find('.message')
  expect(message.text()).toContain('Reminders: 1; logs: 2; settings: 3; skipped logs: 4')
  expect(message.text()).toContain('before-import.json')
  expect(message.classes().includes('message-error')).toBe(warning)
  expect(message.attributes('role')).toBe(warning ? 'alert' : 'status')
  wrapper.unmount()
})

it('rejects malformed backup JSON with a localized message before importing', async () => {
  vi.mocked(confirm).mockResolvedValue(true)
  vi.mocked(open).mockResolvedValue('backup.json')
  const original = vi.mocked(invoke).getMockImplementation()!
  vi.mocked(invoke).mockImplementation((command, args, options) => command === 'read_file' ? Promise.resolve('broken JSON') : original(command, args, options))
  const wrapper = mount(SettingsPage, { global: { stubs: { LanguageSelect: true, MascotStyleSettings: true, NotificationAppearanceSettings: true } } })
  await flushPromises()
  await wrapper.findAll('.data-actions .data-button')[1]!.trigger('click')
  await flushPromises()
  expect(wrapper.find('[role="alert"]').text()).toContain(useI18n().t('operations.invalidBackup'))
  expect(vi.mocked(invoke).mock.calls.some(([command]) => command === 'import_data')).toBe(false)
  wrapper.unmount()
})
