// @vitest-environment jsdom
import type { NotificationPreviewRequest } from './types/notificationPreview'
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import NotificationApp from './NotificationApp.vue'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  hide: vi.fn(),
  sound: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow: () => ({
    hide: mocks.hide,
    listen: async (name: string, callback: (event: { payload: unknown }) => void) => {
      mocks.listeners.set(name, callback)
      return () => mocks.listeners.delete(name)
    },
  }),
}))
vi.mock('./utils/notificationSound', () => ({ playNotificationSound: mocks.sound }))

function request(reminderType: NotificationPreviewRequest['reminderType'] = 'drink'): NotificationPreviewRequest {
  return {
    mascotStyle: 'editorial',
    reminderType,
    settings: { theme: 'light', language: 'zh-CN', notificationDuration: 5, postponeOptions: [5, 10, 15], soundEnabled: false },
  }
}

describe('isolated notification preview', () => {
  let wrapper: ReturnType<typeof mount>
  beforeEach(() => {
    vi.useFakeTimers()
    mocks.invoke.mockReset().mockResolvedValue(null)
    mocks.hide.mockReset().mockResolvedValue(undefined)
    mocks.sound.mockReset()
    localStorage.clear()
  })
  afterEach(() => {
    wrapper?.unmount()
    mocks.listeners.clear()
    vi.useRealTimers()
  })

  it('keeps the countdown running while collapsed and restores it on expansion', async () => {
    wrapper = mount(NotificationApp, { props: { preview: true, previewRequest: request('rest') } })
    await flushPromises()
    await wrapper.get('.complete-button').trigger('click')
    await wrapper.get('.collapse-button').trigger('click')
    expect(wrapper.get('[role="timer"]').text()).toBe('05:00')
    await vi.advanceTimersByTimeAsync(15000)
    expect(wrapper.get('[role="timer"]').text()).toBe('04:45')
    await wrapper.get('.compact-expand').trigger('click')
    expect(wrapper.get('.break-countdown').text()).toBe('04:45')
    await wrapper.get('.finish-button').trigger('click')
    expect(wrapper.emitted('closed')).toHaveLength(1)
    expect(mocks.invoke).not.toHaveBeenCalled()
  })

  it('pauses dismissal while the user is interacting and resumes the remaining time afterwards', async () => {
    wrapper = mount(NotificationApp, { props: { preview: true, previewRequest: request() } })
    await flushPromises()
    await vi.advanceTimersByTimeAsync(2000)
    await wrapper.get('.notification-shell').trigger('mouseenter')
    await vi.advanceTimersByTimeAsync(10000)
    expect(wrapper.find('.notification-shell').exists()).toBe(true)
    await wrapper.get('.notification-shell').trigger('mouseleave')
    await vi.advanceTimersByTimeAsync(2999)
    expect(wrapper.find('.notification-shell').exists()).toBe(true)
    await vi.advanceTimersByTimeAsync(1)
    expect(wrapper.emitted('closed')).toHaveLength(1)
  })

  it.each(['focusin', 'mouseenter'])('dismisses the next notification after closing one during %s', async (interaction) => {
    wrapper = mount(NotificationApp, { props: { preview: true, previewRequest: request() } })
    await flushPromises()
    await wrapper.get('.notification-shell').trigger(interaction)
    await wrapper.get('.skip-button').trigger('click')
    expect(wrapper.find('.notification-shell').exists()).toBe(false)

    await wrapper.setProps({ previewRequest: request() })
    await flushPromises()
    await vi.advanceTimersByTimeAsync(4999)
    expect(wrapper.find('.notification-shell').exists()).toBe(true)
    await vi.advanceTimersByTimeAsync(1)
    expect(wrapper.find('.notification-shell').exists()).toBe(false)
    expect(wrapper.emitted('closed')).toHaveLength(2)
    expect(mocks.invoke).not.toHaveBeenCalled()
  })

  it.each(['soft', 'minimal', 'illustrated'] as const)('previews custom reminders in %s without starting an action', async (appearance) => {
    const custom = request('custom')
    custom.icon = 'custom_leaf'
    custom.settings.notificationAppearance = appearance
    wrapper = mount(NotificationApp, { props: { preview: true, previewRequest: custom } })
    await flushPromises()
    expect(wrapper.find(`.appearance-${appearance}`).exists()).toBe(true)
    if (appearance !== 'minimal')
      expect(wrapper.get('.notification-image').attributes('src')).toContain('custom-leaf.png')
    else
      expect(wrapper.find('.notification-visual').exists()).toBe(false)
    await wrapper.get('.complete-button').trigger('click')
    expect(wrapper.emitted('closed')).toHaveLength(1)
    expect(mocks.invoke).not.toHaveBeenCalled()
  })

  it.each(['.complete-button', '.skip-button', '.snooze-default'])('closes %s without writing reminder data', async (button) => {
    wrapper = mount(NotificationApp, { props: { preview: true, previewRequest: request() } })
    await flushPromises()
    expect(wrapper.get('.notification-image').attributes('src')).toContain('editorial/drink.webp')
    expect(wrapper.get('.notification-tag').text()).toBe('效果预览')
    await wrapper.get(button).trigger('click')
    await vi.advanceTimersByTimeAsync(6000)
    expect(wrapper.find('.notification-shell').exists()).toBe(false)
    expect(wrapper.emitted('closed')).toHaveLength(1)
    expect(mocks.invoke).not.toHaveBeenCalled()
    expect(mocks.sound).not.toHaveBeenCalled()
  })

  it('previews the rest countdown and its matching snooze art without starting a real break', async () => {
    wrapper = mount(NotificationApp, { props: { preview: true, previewRequest: request('rest') } })
    await flushPromises()
    expect(wrapper.get('.notification-image').attributes('src')).toContain('editorial/rest.webp')
    await wrapper.get('.complete-button').trigger('click')
    expect(wrapper.get('.break-countdown').text()).toBe('05:00')
    expect(wrapper.get('.notification-image').attributes('src')).toContain('editorial/snooze.webp')
    await vi.advanceTimersByTimeAsync(300000)
    expect(wrapper.find('.notification-shell').exists()).toBe(false)
    expect(mocks.invoke).not.toHaveBeenCalled()
  })

  it('replaces a preview and resets the timeout when the style changes', async () => {
    wrapper = mount(NotificationApp, { props: { preview: true, previewRequest: request() } })
    await flushPromises()
    await vi.advanceTimersByTimeAsync(4000)
    await wrapper.setProps({ previewRequest: { ...request('eye_care'), mascotStyle: 'watercolor' } })
    await flushPromises()
    expect(wrapper.get('.notification-image').attributes('src')).toContain('watercolor/eye-care.webp')
    await vi.advanceTimersByTimeAsync(1000)
    expect(wrapper.find('.notification-shell').exists()).toBe(true)
    await vi.advanceTimersByTimeAsync(4000)
    expect(wrapper.find('.notification-shell').exists()).toBe(false)
    expect(mocks.invoke).not.toHaveBeenCalled()
  })

  it('starts eye care with matching artwork and updates its remaining progress', async () => {
    wrapper = mount(NotificationApp, { props: { preview: true, previewRequest: request('eye_care') } })
    await flushPromises()
    expect(wrapper.get('.complete-button').text()).toBe('远眺一会儿')
    await wrapper.get('.complete-button').trigger('click')
    expect(wrapper.get('.notification-image').attributes('src')).toContain('editorial/eye-care.webp')
    expect(wrapper.get('[role="timer"]').text()).toBe('00:20')
    await vi.advanceTimersByTimeAsync(10000)
    expect(wrapper.get('[role="timer"]').text()).toBe('00:10')
    expect(wrapper.get('.countdown-fill').attributes('style')).toContain('scaleX(0.5)')
    await wrapper.get('.finish-button').trigger('click')
    expect(wrapper.find('.notification-shell').exists()).toBe(false)
    expect(mocks.invoke).not.toHaveBeenCalled()
  })

  it('switches between illustrated and soft popup layouts while keeping preview actions isolated', async () => {
    const original = request('rest')
    original.settings.notificationAppearance = 'illustrated'
    wrapper = mount(NotificationApp, { props: { preview: true, previewRequest: original } })
    await flushPromises()
    expect(wrapper.find('.appearance-illustrated').exists()).toBe(true)
    await wrapper.get('.complete-button').trigger('click')
    expect(wrapper.get('.break-countdown').text()).toBe('05:00')
    expect(wrapper.get('.notification-image').attributes('src')).toContain('editorial/snooze.webp')
    await wrapper.setProps({ previewRequest: request('drink') })
    await flushPromises()
    expect(wrapper.find('.appearance-illustrated').exists()).toBe(false)
    expect(wrapper.get('.complete-button').text()).toBe('喝好了')
    await wrapper.get('.skip-button').trigger('click')
    expect(wrapper.find('.notification-shell').exists()).toBe(false)
    expect(mocks.invoke).not.toHaveBeenCalled()
  })

  it('gets the first native preview even if the request arrived before the webview loaded', async () => {
    mocks.invoke.mockResolvedValue(request())
    wrapper = mount(NotificationApp, { props: { preview: true } })
    await flushPromises()
    expect(wrapper.get('.notification-title').text()).toBe('喝水提醒')
    expect([...mocks.listeners.keys()]).toEqual(['notification:preview'])
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await flushPromises()
    expect(mocks.hide).toHaveBeenCalledOnce()
    expect(mocks.invoke.mock.calls.map(([command]) => command)).toEqual(['get_notification_preview', 'resize_notification_window'])
  })

  it('does not replay stale startup data over a more recent native preview', async () => {
    let finish!: (value: NotificationPreviewRequest) => void
    mocks.invoke.mockImplementation(command => command === 'get_notification_preview'
      ? new Promise((resolve) => {
          finish = resolve
        })
      : Promise.resolve())
    wrapper = mount(NotificationApp, { props: { preview: true } })
    await flushPromises()
    mocks.listeners.get('notification:preview')!({ payload: { ...request('eye_care'), mascotStyle: 'watercolor' } })
    await flushPromises()
    finish(request())
    await flushPromises()
    expect(wrapper.get('.notification-image').attributes('src')).toContain('watercolor/eye-care.webp')
  })
})
