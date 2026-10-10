// @vitest-environment jsdom
import type { Reminder } from '../types/reminder'
import { mount } from '@vue/test-utils'
import { expect, it } from 'vitest'
import ReminderCard from './ReminderCard.vue'

const reminder: Reminder = {
  id: 'one',
  name: 'Drink',
  message: 'Water',
  reminder_type: 'drink',
  icon: 'drink',
  interval_minutes: 90,
  enabled: true,
  next_trigger: '2026-10-11T04:30:00Z',
  created_at: '',
  updated_at: '',
  break_duration_minutes: 0,
  break_notification_enabled: false,
  action_enabled: false,
  action_title: '',
  action_message: '',
  action_duration_seconds: 0,
  action_completion_mode: 'auto',
}

it('keeps the next clock time for scheduled reminders and replaces it with actionable special states', async () => {
  const wrapper = mount(ReminderCard, { props: { reminder, remainingSeconds: 90, runtime: { state: 'scheduled' } } })
  expect(wrapper.find('.next-trigger').exists()).toBe(true)
  expect(wrapper.find('.special-status').exists()).toBe(false)
  expect(wrapper.get('[role="switch"]').attributes('aria-checked')).toBe('true')
  await wrapper.setProps({ runtime: { state: 'queued' } })
  expect(wrapper.find('.next-trigger').exists()).toBe(false)
  expect(wrapper.get('.special-status').text()).not.toBe('')
  await wrapper.setProps({ runtime: { state: 'paused' }, globallyPaused: true })
  expect(wrapper.find('.next-trigger').exists()).toBe(false)
  expect(wrapper.find('.special-status').exists()).toBe(false)
  expect(wrapper.get('.countdown-value').text()).toBe('1:30')
  expect(wrapper.get('[role="switch"]').attributes('aria-checked')).toBe('true')
  wrapper.unmount()
})
