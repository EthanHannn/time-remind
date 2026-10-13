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
vi.mock('./i18n', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./i18n')>()
  return {
    ...actual,
    loadLanguage: async () => {},
    useI18n: () => ({ ...actual.useI18n(), locale: ref('en-US'), t: (key: string) => key }),
  }
})
vi.mock('./utils/notificationSound', () => ({ playNotificationSound: mocks.sound }))
vi.mock('./utils/reminderVisuals', () => ({
  appIconMain: '/test-app-icon.png',
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

  it('keeps a failed completion visible and retries without starting a second action', async () => {
    await wrapper.find('.complete-button').trigger('click')
    await flushPromises()
    mocks.invoke.mockRejectedValueOnce('disk full')
    await wrapper.find('.finish-button').trigger('click')
    await flushPromises()
    expect(wrapper.find('.break-countdown').exists()).toBe(true)
    expect(wrapper.find('[role="alert"]').text()).toContain('feedback.failed')
    expect(wrapper.find('.finish-button').attributes('disabled')).toBeUndefined()
    await wrapper.find('.finish-button').trigger('click')
    await flushPromises()
    expect(wrapper.find('.notification-shell').exists()).toBe(false)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'respond_reminder')).toHaveLength(1)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'release_notification')).toHaveLength(2)
  })

  it('uses the remaining duration returned by the backend', async () => {
    mocks.invoke.mockResolvedValueOnce({ remaining_seconds: 3 })
    await wrapper.find('.complete-button').trigger('click')
    await flushPromises()
    expect(wrapper.find('.break-countdown').text()).toBe('00:03')
    await vi.advanceTimersByTimeAsync(3000)
    expect(mocks.invoke).toHaveBeenCalledWith('release_notification', {
      reminderId: 'drink',
      notificationId: 'instance-1',
      finishBreakNow: false,
    })
  })

  it.each([
    ['soft', 272],
    ['minimal', 264],
    ['illustrated', 296],
  ])('starts an action in %s without resizing or remounting the native popup', async (appearance, height) => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'get_all_settings')
        return { theme: 'light', notification_appearance: appearance }
      if (command === 'respond_reminder')
        return { remaining_seconds: 10 }
      if (command === 'get_reminders')
        return []
    })
    emit('settings:changed')
    await flushPromises()
    const shell = wrapper.get('.notification-shell').element
    const resizes = () => mocks.invoke.mock.calls.filter(([command]) => command === 'resize_notification_window')
    expect(resizes().slice(-1)[0]?.[1]).toEqual({ width: 400, height })
    const count = resizes().length
    await wrapper.get('.complete-button').trigger('click')
    await flushPromises()
    expect(wrapper.get('.break-countdown').text()).toBe('00:10')
    expect(wrapper.get('.notification-shell').element).toBe(shell)
    expect(resizes()).toHaveLength(count)
    await wrapper.get('.collapse-button').trigger('click')
    await flushPromises()
    expect(resizes().slice(-1)[0]?.[1]).toEqual({ width: 340, height: 96 })
    await wrapper.get('.compact-expand').trigger('click')
    await flushPromises()
    expect(resizes().slice(-1)[0]?.[1]).toEqual({ width: 400, height })
  })

  it('refreshes visible built-in copy after a language change without restarting the activity', async () => {
    await wrapper.find('.complete-button').trigger('click')
    await flushPromises()
    await vi.advanceTimersByTimeAsync(3000)
    const remaining = wrapper.find('.break-countdown').text()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'get_all_settings')
        return { language: 'fr-FR', notification_duration: '5' }
      if (command === 'get_reminders')
        return [{ id: 'drink', name: 'Boire', message: 'Buvez de l’eau', action_title: 'Petite pause', action_message: 'Détendez-vous' }]
    })
    emit('settings:changed')
    await flushPromises()
    expect(wrapper.find('.notification-title').text()).toBe('Petite pause')
    expect(wrapper.find('.notification-message').text()).toBe('Détendez-vous')
    expect(wrapper.find('.break-countdown').text()).toBe(remaining)
    expect(mocks.invoke.mock.calls.filter(([name]) => name === 'respond_reminder')).toHaveLength(1)
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
