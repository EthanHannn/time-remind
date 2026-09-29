// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { loadLanguage, useI18n } from './index'

const invoke = vi.hoisted(() => vi.fn(async () => ({})))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

describe('language selection', () => {
  beforeEach(() => {
    localStorage.clear()
    invoke.mockClear()
  })

  it.each([
    ['es-MX', 'es-ES'],
    ['pt-PT', 'pt-BR'],
    ['id', 'id-ID'],
    ['it-CH', 'it-IT'],
    ['ru', 'ru-RU'],
    ['tr', 'tr-TR'],
    ['ar-SA', 'ar'],
    ['hi', 'hi-IN'],
  ])('loads the supported locale for %s', async (input, expected) => {
    await loadLanguage({ language: input })
    expect(useI18n().language.value).toBe(expected)
    expect(document.documentElement.lang).toBe(expected)
  })

  it('persists Arabic, restores RTL on reload, and returns to LTR when switching', async () => {
    await useI18n().setLanguage('ar')
    expect(invoke).toHaveBeenCalledWith('save_setting', { key: 'language', value: 'ar' })
    expect(localStorage.getItem('app-language')).toBe('ar')
    await loadLanguage({})
    expect(document.documentElement.dir).toBe('rtl')
    expect(useI18n().t('app.runningCount', { count: 3 })).toContain('3')
    await useI18n().setLanguage('es-ES')
    expect(document.documentElement.dir).toBe('ltr')
  })
})
