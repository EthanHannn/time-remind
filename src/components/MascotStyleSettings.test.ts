// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import SettingsPage from './SettingsPage.vue'

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), native: true, settings: {} as Record<string, string> }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke, isTauri: () => mocks.native }))
vi.mock('@tauri-apps/plugin-autostart', () => ({ disable: vi.fn(), enable: vi.fn(), isEnabled: async () => false }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ confirm: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('../utils/notificationSound', () => ({ playNotificationSound: vi.fn() }))

describe('cat style settings', () => {
  let wrapper: ReturnType<typeof mount>
  beforeEach(() => {
    mocks.native = true
    localStorage.clear()
    mocks.settings = { theme: 'light', language: 'zh-CN', sound_enabled: 'false' }
    mocks.invoke.mockReset().mockImplementation(async (command: string, payload?: { key?: string, value?: string }) => {
      if (command === 'get_all_settings')
        return { ...mocks.settings }
      if (command === 'save_setting')
        mocks.settings[payload!.key!] = payload!.value!
      if (command === 'get_platform_capabilities') {
        return { platform: 'windows', supportsAutostart: true, supportsSilentStart: true, supportsFullscreenDetection: true, supportsLockDetection: true, supportsTray: true, isVerifiedReleasePlatform: true }
      }
    })
  })
  afterEach(() => wrapper?.unmount())

  it('keeps classic for existing installs, persists a new selection, and restores it on reopening', async () => {
    wrapper = mount(SettingsPage)
    await flushPromises()
    expect((wrapper.get('.mascot-options input[value="classic"]').element as HTMLInputElement).checked).toBe(true)
    await wrapper.get('input[value="editorial"]').setValue()
    await flushPromises()
    expect(mocks.settings.mascot_style).toBe('editorial')
    expect(JSON.parse(localStorage.getItem('app-settings')!).mascotStyle).toBe('editorial')
    wrapper.unmount()
    wrapper = mount(SettingsPage)
    await flushPromises()
    expect((wrapper.get('input[value="editorial"]').element as HTMLInputElement).checked).toBe(true)
  })

  it('previews the selected style and state immediately, even before settings finish saving', async () => {
    wrapper = mount(SettingsPage)
    await flushPromises()
    await wrapper.get('input[value="watercolor"]').setValue()
    await wrapper.get('.mascot-state-select').setValue('eye_care')
    await wrapper.get('.mascot-preview-button').trigger('click')
    await flushPromises()
    expect(mocks.invoke).toHaveBeenCalledWith('preview_notification', {
      request: expect.objectContaining({ mascotStyle: 'watercolor', reminderType: 'eye_care', settings: expect.objectContaining({ soundEnabled: false }) }),
    })
    expect(mocks.invoke.mock.calls.some(([command]) => ['respond_reminder', 'postpone_reminder', 'create_reminder'].includes(command))).toBe(false)
  })

  it('falls back to classic for an unsupported style in a backup', async () => {
    mocks.settings.mascot_style = 'removed-theme'
    wrapper = mount(SettingsPage)
    await flushPromises()
    expect((wrapper.get('.mascot-options input[value="classic"]').element as HTMLInputElement).checked).toBe(true)
  })

  it('saves and restores popup appearance independently of the cat style and includes it in previews', async () => {
    wrapper = mount(SettingsPage)
    await flushPromises()
    await wrapper.get('.appearance-settings input[value="classic"]').setValue()
    await flushPromises()
    expect(mocks.settings.notification_appearance).toBe('classic')
    expect(JSON.parse(localStorage.getItem('app-settings')!).notificationAppearance).toBe('classic')
    await wrapper.get('.mascot-preview-button').trigger('click')
    await flushPromises()
    expect(mocks.invoke).toHaveBeenCalledWith('preview_notification', {
      request: expect.objectContaining({ settings: expect.objectContaining({ notificationAppearance: 'classic' }) }),
    })
    wrapper.unmount()
    wrapper = mount(SettingsPage)
    await flushPromises()
    expect((wrapper.get('.appearance-settings input[value="classic"]').element as HTMLInputElement).checked).toBe(true)
    expect((wrapper.get('.mascot-options input[value="classic"]').element as HTMLInputElement).checked).toBe(true)
  })

  it('shows a useful error when the native preview cannot open', async () => {
    wrapper = mount(SettingsPage)
    await flushPromises()
    const error = vi.spyOn(console, 'error').mockImplementation(() => {})
    mocks.invoke.mockImplementationOnce(async () => {
      throw new Error('window missing')
    })
    await wrapper.get('.mascot-preview-button').trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toBe('暂时无法打开预览，请重试。')
    error.mockRestore()
  })

  it('opens and dismisses the same preview in a browser dialog', async () => {
    mocks.native = false
    const showModal = vi.fn(function (this: HTMLDialogElement) {
      this.setAttribute('open', '')
    })
    const close = vi.fn(function (this: HTMLDialogElement) {
      this.removeAttribute('open')
    })
    Object.defineProperties(HTMLDialogElement.prototype, {
      showModal: { configurable: true, value: showModal },
      close: { configurable: true, value: close },
    })
    wrapper = mount(SettingsPage)
    await flushPromises()
    await wrapper.get('input[value="watercolor"]').setValue()
    await wrapper.get('.mascot-preview-button').trigger('click')
    await flushPromises()
    expect(wrapper.find('dialog[open]').exists()).toBe(true)
    expect(wrapper.get('.notification-image').attributes('src')).toContain('watercolor/drink.webp')
    await wrapper.get('.skip-button').trigger('click')
    await flushPromises()
    expect(wrapper.find('dialog[open]').exists()).toBe(false)
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'preview_notification')).toBe(false)
    Reflect.deleteProperty(HTMLDialogElement.prototype, 'showModal')
    Reflect.deleteProperty(HTMLDialogElement.prototype, 'close')
  })
})
