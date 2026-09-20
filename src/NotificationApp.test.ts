// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import NotificationApp from './NotificationApp.vue'

const mocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  invoke: vi.fn(),
  hide: vi.fn(),
  sound: vi.fn(),
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
vi.mock('./i18n', () => ({
  loadLanguage: async () => {},
  useI18n: () => ({ locale: ref('en'), t: (key: string) => key }),
}))
vi.mock('./utils/notificationSound', () => ({ playNotificationSound: mocks.sound }))
vi.mock('./utils/reminderVisuals', () => ({
  getLocalizedReminderVisual: () => ({ label: 'Drink', accent: '#abc', accentSoft: '#def', borderSoft: '#123', iconText: 'D' }),
}))

function emit(name: string, payload: unknown = {}) {
  const listener = mocks.listeners.get(name)
  expect(listener, `listener for ${name}`).toBeDefined()
  listener!({ payload })
}

const notification = { queue_revision: 1, notification_id: 'instance-1', reminder_id: 'drink', name: 'Drink', action_enabled: true, action_duration_seconds: 10 }

describe('notification cancellation', () => {
  let wrapper: ReturnType<typeof mount>

  beforeEach(async () => {
    vi.useFakeTimers()
    mocks.invoke.mockReset().mockImplementation(async (command: string) => {
      return command === 'get_all_settings' ? { theme: 'light', notification_duration: '5' } : undefined
    })
    mocks.hide.mockReset().mockResolvedValue(undefined)
    mocks.sound.mockReset()
    wrapper = mount(NotificationApp)
    await flushPromises()
    emit('notification:show', notification)
    await flushPromises()
  })

  afterEach(() => {
    wrapper.unmount()
    mocks.listeners.clear()
    vi.useRealTimers()
  })

  it('cancels a hidden notification without recording a timeout', async () => {
    emit('notification:queue-updated', { queue_revision: 2, current_reminder_id: null, pending_count: 0 })
    await vi.advanceTimersByTimeAsync(6000)
    expect(wrapper.find('.notification-shell').exists()).toBe(false)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'respond_reminder')).toHaveLength(0)
  })

  it('stops a cancelled break countdown', async () => {
    await wrapper.find('.complete-button').trigger('click')
    await flushPromises()
    expect(wrapper.find('.break-countdown').exists()).toBe(true)
    emit('notification:queue-updated', { queue_revision: 2, current_reminder_id: null })
    await vi.advanceTimersByTimeAsync(11000)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'release_notification')).toHaveLength(0)
  })

  it('does not let an old response close a new notification', async () => {
    let finish!: () => void
    mocks.invoke.mockImplementationOnce(() => new Promise<void>((resolve) => {
      finish = resolve
    }))
    await wrapper.find('.skip-button').trigger('click')
    emit('notification:queue-updated', { queue_revision: 2, current_reminder_id: null })
    emit('notification:show', { ...notification, queue_revision: 3, notification_id: 'instance-2', name: 'New drink' })
    await flushPromises()
    finish()
    await flushPromises()
    expect(wrapper.find('.notification-title').text()).toBe('New drink')
    expect(mocks.hide).not.toHaveBeenCalled()
  })

  it('does not restart a timer when cancelled settings finish loading', async () => {
    let finish!: (settings: Record<string, string>) => void
    mocks.invoke.mockImplementationOnce(() => new Promise((resolve) => {
      finish = resolve
    }))
    emit('notification:show', { ...notification, queue_revision: 3 })
    emit('notification:queue-updated', { queue_revision: 2, current_reminder_id: null })
    emit('notification:queue-updated', { queue_revision: 4, current_reminder_id: null })
    finish({ theme: 'light', notification_duration: '5' })
    await vi.advanceTimersByTimeAsync(6000)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'respond_reminder')).toHaveLength(0)
  })

  it('starts a fresh timeout after cancellation and resume', async () => {
    await vi.advanceTimersByTimeAsync(4000)
    emit('notification:queue-updated', { queue_revision: 2, current_reminder_id: null })
    emit('notification:show', { ...notification, queue_revision: 3 })
    await flushPromises()
    await vi.advanceTimersByTimeAsync(1000)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'respond_reminder')).toHaveLength(0)
    await vi.advanceTimersByTimeAsync(4000)
    expect(mocks.invoke).toHaveBeenCalledWith('respond_reminder', {
      reminderId: 'drink',
      notificationId: 'instance-1',
      action: 'timeout',
      holdNotification: false,
    })
  })

  it('ignores an old empty snapshot after a new notification is shown', async () => {
    emit('notification:queue-updated', { queue_revision: 0, current_reminder_id: null, pending_count: 1 })
    await flushPromises()
    expect(wrapper.find('.notification-shell').exists()).toBe(true)
    await vi.advanceTimersByTimeAsync(5000)
    expect(mocks.invoke).toHaveBeenCalledWith('respond_reminder', {
      notificationId: 'instance-1',
      reminderId: 'drink',
      action: 'timeout',
      holdNotification: false,
    })
  })

  it('does not resurrect a notification when show arrives after its cancellation', async () => {
    emit('notification:queue-updated', { queue_revision: 2, current_reminder_id: null })
    emit('notification:show', notification)
    await vi.advanceTimersByTimeAsync(6000)
    expect(wrapper.find('.notification-shell').exists()).toBe(false)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'respond_reminder')).toHaveLength(0)
  })
})
