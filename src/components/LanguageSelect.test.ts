// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import LanguageSelect from './LanguageSelect.vue'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

describe('language picker', () => {
  afterEach(() => vi.restoreAllMocks())

  it('exposes all languages and reaches the final option by keyboard without saving intermediate choices', async () => {
    const scroll = vi.fn()
    const original = HTMLElement.prototype.scrollIntoView
    HTMLElement.prototype.scrollIntoView = scroll
    const wrapper = mount(LanguageSelect, { props: { id: 'language-test', modelValue: 'zh-CN' }, attachTo: document.body })
    try {
      const trigger = wrapper.get('[role="combobox"]')
      await trigger.trigger('keydown', { key: 'ArrowDown' })
      expect(wrapper.findAll('[role="option"]')).toHaveLength(19)
      await trigger.trigger('keydown', { key: 'End' })
      await flushPromises()
      expect(scroll).toHaveBeenCalled()
      expect(wrapper.emitted('update:modelValue')).toBeUndefined()
      await trigger.trigger('keydown', { key: 'Enter' })
      expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['hi-IN'])
      expect(trigger.attributes('aria-expanded')).toBe('false')
    }
    finally {
      wrapper.unmount()
      HTMLElement.prototype.scrollIntoView = original
    }
  })

  it('dismisses with Escape without changing the language', async () => {
    const wrapper = mount(LanguageSelect, { props: { id: 'language-dismiss', modelValue: 'zh-CN' } })
    try {
      const trigger = wrapper.get('[role="combobox"]')
      await trigger.trigger('click')
      await trigger.trigger('keydown', { key: 'Escape' })
      expect(trigger.attributes('aria-expanded')).toBe('false')
      expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    }
    finally {
      wrapper.unmount()
    }
  })
})
