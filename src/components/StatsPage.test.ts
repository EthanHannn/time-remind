// @vitest-environment jsdom
import { invoke } from '@tauri-apps/api/core'
import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vitest'
import { useI18n } from '../i18n'
import StatsPage from './StatsPage.vue'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }))
const snapshot = (count: number) => ({ daily: [{ reminder_id: 'one', name: 'One', reminder_type: 'drink', icon: 'drink', completed_count: count, total_count: count }], trend: [], streaks: [] })
const render = () => mount(StatsPage, { global: { stubs: { CompletionDoughnut: true, TrendChart: true } } })

it('ignores an older statistics request when changing the date range quickly', async () => {
  vi.mocked(invoke).mockResolvedValueOnce(snapshot(1))
  const wrapper = render()
  await flushPromises()
  let resolveOld!: (value: unknown) => void
  vi.mocked(invoke).mockImplementationOnce(() => new Promise((resolve) => {
    resolveOld = resolve
  })).mockResolvedValueOnce(snapshot(3))
  await wrapper.findAll('.trend-button')[1]!.trigger('click')
  await wrapper.findAll('.trend-button')[0]!.trigger('click')
  await flushPromises()
  resolveOld(snapshot(2))
  await flushPromises()
  expect(wrapper.find('.stat-foot').text()).toContain('3/3')
  wrapper.unmount()
})

it('distinguishes a load failure from no statistics and supports retry', async () => {
  vi.mocked(invoke).mockRejectedValueOnce('disk unavailable').mockResolvedValueOnce(snapshot(1))
  const wrapper = render()
  await flushPromises()
  expect(wrapper.find('[role="alert"]').text()).toContain(useI18n().t('operations.unexpected'))
  expect(wrapper.find('.stats-empty').exists()).toBe(false)
  await wrapper.find('[role="alert"] button').trigger('click')
  await flushPromises()
  expect(wrapper.find('.stat-foot').text()).toContain('1/1')
  wrapper.unmount()
})
