// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vitest'
import { createReminder, toggleReminder } from './api/reminder'
import App from './App.vue'
import AddReminderForm from './components/AddReminderForm.vue'

const mocks = vi.hoisted(() => ({
  overview: vi.fn(),
  listeners: {} as Record<string, (event: any) => void>,
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue({}) }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name, callback) => {
    mocks.listeners[name] = callback
    return () => {}
  }),
}))
vi.mock('./api/reminder', () => ({
  getReminderOverview: mocks.overview,
  createReminder: vi.fn(),
  deleteReminder: vi.fn(),
  toggleReminder: vi.fn(),
}))
vi.mock('./i18n', () => ({ loadLanguage: vi.fn(), useI18n: () => ({ t: (key: string) => key }) }))
vi.mock('./utils/reminderVisuals', () => ({ emptyReminders: '' }))

function snapshot(paused: boolean, seconds: number) {
  return { reminders: [{ id: 'one', enabled: true }], all_paused: paused, countdowns: { one: seconds } }
}
function render() {
  return mount(App, { global: { stubs: {
    AppTitleBar: true,
    AppResizeHandles: true,
    SettingsPage: true,
    StatsPage: true,
    AddReminderForm: true,
    ConfirmDialog: true,
    ReminderCard: { name: 'ReminderCard', props: ['globallyPaused', 'remainingSeconds'], template: '<div class="card">{{ globallyPaused }}:{{ remainingSeconds }}</div>' },
  } } })
}
beforeEach(() => {
  vi.clearAllMocks()
  mocks.overview.mockReset()
  mocks.listeners = {}
})

it('guards repeated create requests and keeps a failed form available to retry', async () => {
  mocks.overview.mockResolvedValue(snapshot(false, 60))
  let reject!: (reason: string) => void
  vi.mocked(createReminder).mockImplementationOnce(() => new Promise((_, fail) => {
    reject = fail
  }))
  const wrapper = render()
  await flushPromises()
  await wrapper.find('.header-button-primary').trigger('click')
  const form = wrapper.findComponent(AddReminderForm)
  form.vm.$emit('submit', { name: 'Custom' })
  form.vm.$emit('submit', { name: 'Custom' })
  await flushPromises()
  expect(createReminder).toHaveBeenCalledTimes(1)
  expect(form.attributes('pending')).toBe('true')
  reject('disk full')
  await flushPromises()
  expect(form.attributes('error')).toBe('feedback.failed')
  expect(wrapper.findComponent(AddReminderForm).exists()).toBe(true)
  vi.mocked(createReminder).mockResolvedValueOnce({} as any)
  form.vm.$emit('submit', { name: 'Custom' })
  await flushPromises()
  expect(createReminder).toHaveBeenCalledTimes(2)
  expect(wrapper.findComponent(AddReminderForm).exists()).toBe(false)
  wrapper.unmount()
})

it('shows a load failure instead of an empty list and allows retry', async () => {
  mocks.overview.mockRejectedValueOnce('offline').mockResolvedValue(snapshot(false, 60))
  const wrapper = render()
  await flushPromises()
  expect(wrapper.find('[role="alert"]').text()).toContain('feedback.failed')
  expect(wrapper.find('.empty-state').exists()).toBe(false)
  await wrapper.find('[role="alert"] button').trigger('click')
  await flushPromises()
  expect(wrapper.find('.card').exists()).toBe(true)
  wrapper.unmount()
})

it('keeps the enabled state when a toggle fails and reports the failure', async () => {
  mocks.overview.mockResolvedValue(snapshot(false, 60))
  vi.mocked(toggleReminder).mockRejectedValueOnce('offline')
  const wrapper = render()
  await flushPromises()
  wrapper.findComponent({ name: 'ReminderCard' }).vm.$emit('toggle', 'one')
  await flushPromises()
  expect(wrapper.find('[role="alert"]').text()).toContain('feedback.failed')
  expect(wrapper.find('.card').text()).toBe('false:60')
  wrapper.unmount()
})
it('freezes ticks during pause and updates on resume', async () => {
  mocks.overview.mockResolvedValue(snapshot(true, 300))
  const wrapper = render()
  await flushPromises()
  expect(wrapper.find('h1').text()).toBe('reminder.paused')
  mocks.listeners['timer:tick']({ payload: { reminder_id: 'one', remaining_seconds: 0 } })
  await flushPromises()
  expect(wrapper.find('.card').text()).toBe('true:300')
  mocks.overview.mockResolvedValue(snapshot(false, 300))
  mocks.listeners['reminders:changed']({})
  await flushPromises()
  mocks.listeners['timer:tick']({ payload: { reminder_id: 'one', remaining_seconds: 299 } })
  await flushPromises()
  expect(wrapper.find('.card').text()).toBe('false:299')
  wrapper.unmount()
})
it('ignores an older overview that resolves after a pause update', async () => {
  let resolveOld!: (value: ReturnType<typeof snapshot>) => void
  mocks.overview.mockReturnValueOnce(new Promise((resolve) => {
    resolveOld = resolve
  }))
  const wrapper = render()
  await flushPromises()
  mocks.overview.mockResolvedValue(snapshot(true, 300))
  mocks.listeners['reminders:changed']({})
  await flushPromises()
  resolveOld(snapshot(false, 0))
  await flushPromises()
  expect(wrapper.find('.card').text()).toBe('true:300')
  wrapper.unmount()
})

it('replaces a stale countdown with authoritative state and rejects older state and ticks', async () => {
  mocks.overview.mockResolvedValue(snapshot(false, 1))
  const wrapper = render()
  await flushPromises()
  mocks.listeners['reminders:state']({ payload: { revision: 3, all_paused: false, countdowns: { one: 0 }, states: { one: { state: 'notifying' } } } })
  mocks.listeners['timer:tick']({ payload: { reminder_id: 'one', remaining_seconds: 1 } })
  mocks.listeners['reminders:state']({ payload: { revision: 2, all_paused: false, countdowns: { one: 1 } } })
  await flushPromises()
  expect(wrapper.find('.card').text()).toBe('false:0')
  mocks.listeners['reminders:state']({ payload: { revision: 4, all_paused: false, countdowns: { one: 300 }, states: { one: { state: 'dnd' } }, suppression: { state: 'dnd', reason: 'temporary' } } })
  await flushPromises()
  expect(wrapper.find('h1').text()).toBe('settings.dnd')
  wrapper.unmount()
})
