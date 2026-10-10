// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { afterEach, expect, it } from 'vitest'
import SnoozeControl from './SnoozeControl.vue'

let wrapper: ReturnType<typeof mount>
afterEach(() => wrapper?.unmount())

it('offers a direct five-minute snooze and preserves every configured duration', async () => {
  wrapper = mount(SnoozeControl, { props: { options: [15, 5, 10].map(minutes => ({ minutes, label: `${minutes} min` })) } })
  await wrapper.get('.snooze-default').trigger('click')
  expect(wrapper.emitted('postpone')).toEqual([[5]])
  await wrapper.get('[aria-haspopup="menu"]').trigger('click')
  expect(wrapper.findAll('[role="menuitem"]').map(item => item.text())).toEqual(['15 min', '5 min', '10 min'])
  await wrapper.findAll('[role="menuitem"]')[2]!.trigger('click')
  expect(wrapper.emitted('postpone')).toEqual([[5], [10]])
  expect(wrapper.find('[role="menu"]').exists()).toBe(false)
})

it('uses a saved custom duration when five minutes is not configured', async () => {
  wrapper = mount(SnoozeControl, { props: { options: [{ minutes: 8, label: '8 min' }] } })
  await wrapper.get('.snooze-default').trigger('click')
  expect(wrapper.emitted('postpone')).toEqual([[8]])
  expect(wrapper.find('[aria-haspopup="menu"]').exists()).toBe(false)
})

it('supports arrow navigation and restores focus on Escape without snoozing', async () => {
  wrapper = mount(SnoozeControl, { attachTo: document.body, props: { options: [5, 10, 15].map(minutes => ({ minutes, label: `${minutes} min` })) } })
  const trigger = wrapper.get('[aria-haspopup="menu"]')
  await trigger.trigger('click')
  const options = wrapper.findAll('[role="menuitem"]')
  expect(document.activeElement).toBe(options[0]!.element)
  await options[0]!.trigger('keydown', { key: 'ArrowDown' })
  expect(document.activeElement).toBe(options[1]!.element)
  await options[1]!.trigger('keydown', { key: 'Escape' })
  expect(document.activeElement).toBe(trigger.element)
  expect(wrapper.find('[role="menu"]').exists()).toBe(false)
  expect(wrapper.emitted('postpone')).toBeUndefined()
})
