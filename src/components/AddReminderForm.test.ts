// @vitest-environment jsdom
import type { Reminder } from '../types/reminder'
import { mount } from '@vue/test-utils'
import { afterEach, expect, it } from 'vitest'
import AddReminderForm from './AddReminderForm.vue'

const reminder: Reminder = {
  id: 'custom',
  name: 'Desk',
  message: 'Clear the desk',
  reminder_type: 'custom',
  icon: 'custom_leaf',
  interval_minutes: 45,
  enabled: false,
  next_trigger: null,
  created_at: '',
  updated_at: '',
  break_duration_minutes: 0,
  break_notification_enabled: false,
  action_enabled: true,
  action_title: 'Organize',
  action_message: 'One item at a time',
  action_duration_seconds: 90,
  action_completion_mode: 'manual',
}
let wrapper: ReturnType<typeof mount>
afterEach(() => wrapper?.unmount())

it('loads existing values and saves an edit without changing its icon, schedule, or completion mode', async () => {
  wrapper = mount(AddReminderForm, { props: { reminder } })
  expect((wrapper.get('[name="name"]').element as HTMLInputElement).value).toBe('Desk')
  await wrapper.get('[name="name"]').setValue('Desk updated')
  await wrapper.get('form').trigger('submit')
  const [data] = wrapper.emitted('submit')![0]!
  expect(data).toMatchObject({ name: 'Desk updated', icon: 'custom_leaf', interval_minutes: 45, action_duration_seconds: 90, action_completion_mode: 'manual' })
  expect(data).not.toHaveProperty('enabled')
  expect(reminder.name).toBe('Desk')
})

it('keeps failed edits available and blocks duplicate submissions while saving', async () => {
  wrapper = mount(AddReminderForm, { props: { reminder, pending: true } })
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('submit')).toBeUndefined()
  await wrapper.setProps({ pending: false, error: 'Disk full' })
  expect(wrapper.get('[role="alert"]').text()).toBe('Disk full')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('submit')).toHaveLength(1)
})
