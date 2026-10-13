// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { formatClock, formatNumber, formatUnit } from './format'
import { loadLanguage, useI18n } from './index'
import { languageOptions, messages } from './messages'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
afterEach(() => vi.restoreAllMocks())

describe('localized values and feedback', () => {
  it('uses English singulars and Russian and Arabic plural categories', async () => {
    const { t } = useI18n()
    await loadLanguage({ language: 'en-US' })
    expect(t('notification.pendingCount', { count: 1 })).toBe('1 more reminder waiting')
    expect(t('notification.pendingCount', { count: 2 })).toBe('2 more reminders waiting')
    expect(t('app.runningCount', { count: 1 })).toBe('1 reminder running')
    await loadLanguage({ language: 'ru-RU' })
    for (const [count, ending] of [[1, 'напоминание'], [2, 'напоминания'], [5, 'напоминаний'], [21, 'напоминание'], [22, 'напоминания']] as const)
      expect(t('notification.pendingCount', { count })).toContain(ending)
    await loadLanguage({ language: 'ar' })
    const variants = [0, 1, 2, 3, 11, 100].map(count => t('notification.pendingCount', { count }))
    expect(new Set(variants).size).toBe(6)
    expect(variants[2]).toContain('تذكيران')
    expect(variants[3]).toContain(new Intl.NumberFormat('ar').format(3))
  })

  it('formats units, percentages and countdowns consistently', () => {
    expect(formatUnit('en-US', 1, 'minute')).toBe('1 minute')
    expect(formatUnit('en-US', 2, 'minute')).toBe('2 minutes')
    expect(formatUnit('ru-RU', 5, 'second')).toBe('5 секунд')
    expect(formatClock('en-US', 3661)).toBe('1:01:01')
    expect(formatClock('en-US', 3661, true)).toBe('61:01')
    const digits = new Intl.NumberFormat('ar', { minimumIntegerDigits: 2, useGrouping: false })
    expect(formatClock('ar', 65, true)).toBe(`${digits.format(1)}:${digits.format(5)}`)
    expect(formatNumber('ar', 0.75, { style: 'percent' })).toBe(new Intl.NumberFormat('ar', { style: 'percent' }).format(0.75))
  })

  it('localizes structured errors and import counts for every language without leaking raw errors', async () => {
    const log = vi.spyOn(console, 'error').mockImplementation(() => {})
    const { t, formatError } = useI18n()
    for (const { value: language } of languageOptions) {
      await loadLanguage({ language })
      expect(formatError('数据库连接失败 / access denied')).toBe(messages[language].operations.unexpected)
      expect(formatError('time-remind:{"code":"unknown","params":{}}')).toBe(messages[language].operations.unexpected)
      expect(formatError('time-remind:invalid')).toBe(messages[language].operations.unexpected)
      expect(formatError(new SyntaxError('Unexpected token'))).toBe(messages[language].operations.unexpected)
      expect(formatError('time-remind:{"code":"invalidBackup","params":{}}')).toBe(messages[language].operations.invalidBackup)
      expect(formatError(new DOMException('previous language', 'TimeoutError'))).toBe(messages[language].feedback.loadTimeout)
      const duration = formatError('time-remind:{"code":"actionDuration","params":{"min":1,"max":7200}}')
      expect(duration).toContain(formatNumber(language, 7200))
      expect(duration).not.toContain('{')
      expect(formatError('time-remind:{"code":"backupTooLarge","params":{"max":64}}')).toContain(formatNumber(language, 64))
      const result = t('operations.importSummary', { mode: t('settings.mergeImport'), reminders: 1, logs: 21, settings: 3, skipped: 0, path: 'backup.json' })
      expect(result).not.toMatch(/\{\w+\}/)
      expect(result).toContain('backup.json')
    }
    expect(log).toHaveBeenCalled()
  })
})
