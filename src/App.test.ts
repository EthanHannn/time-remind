// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vitest'
import App from './App.vue'

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
    ReminderCard: { props: ['globallyPaused', 'remainingSeconds'], template: '<div class="card">{{ globallyPaused }}:{{ remainingSeconds }}</div>' },
  } } })
}
beforeEach(() => {
  mocks.overview.mockReset()
  mocks.listeners = {}
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
