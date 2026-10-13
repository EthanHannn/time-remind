import type { Language } from './messages'

export function formatNumber(language: Language, value: number, options?: Intl.NumberFormatOptions) {
  return new Intl.NumberFormat(language, options).format(value)
}

export function formatUnit(language: Language, value: number, unit: 'minute' | 'second' | 'day') {
  return formatNumber(language, value, { style: 'unit', unit, unitDisplay: 'long' })
}

export function formatClock(language: Language, seconds: number, paddedMinutes = false) {
  const safe = Math.max(0, Math.floor(seconds))
  const parts = safe >= 3600 && !paddedMinutes
    ? [Math.floor(safe / 3600), Math.floor(safe / 60) % 60, safe % 60]
    : [Math.floor(safe / 60), safe % 60]
  return parts.map((part, index) => formatNumber(language, part, {
    useGrouping: false,
    minimumIntegerDigits: index > 0 || paddedMinutes ? 2 : 1,
  })).join(':')
}

type PluralForms = Partial<Record<Intl.LDMLPluralRule, string>>
// Languages with invariant wording use their regular message. These variants
// cover the inflected phrases; duration units use Intl's CLDR unit patterns.
const pending: Partial<Record<Language, PluralForms>> = {
  'en-US': { one: '{count} more reminder waiting' },
  'fr-FR': { one: 'Encore {count} rappel en attente' },
  'de-DE': { one: 'Noch {count} Erinnerung in der Warteschlange' },
  'es-ES': { one: '{count} recordatorio más en espera' },
  'pt-BR': { one: 'Mais {count} lembrete aguardando', other: 'Mais {count} lembretes aguardando' },
  'it-IT': { one: '{count} altro promemoria in attesa' },
  'ru-RU': { one: 'Ожидается ещё {count} напоминание', few: 'Ожидаются ещё {count} напоминания', many: 'Ожидаются ещё {count} напоминаний', other: 'Ожидается ещё {count} напоминания' },
  'ar': { zero: 'لا توجد تذكيرات أخرى في الانتظار', one: 'تذكير آخر في الانتظار', two: 'تذكيران آخران في الانتظار', few: '{count} تذكيرات أخرى في الانتظار', many: '{count} تذكيرًا آخر في الانتظار', other: '{count} تذكير آخر في الانتظار' },
  'hi-IN': { one: '{count} और रिमाइंडर प्रतीक्षा में है', other: '{count} और रिमाइंडर प्रतीक्षा में हैं' },
}
const snooze: Partial<Record<Language, PluralForms>> = {
  'de-DE': { one: 'In {minutes} Minute' },
  'ru-RU': { one: 'Через {minutes} минуту', few: 'Через {minutes} минуты', many: 'Через {minutes} минут', other: 'Через {minutes} минуты' },
  'ar': { one: 'بعد دقيقة واحدة', two: 'بعد دقيقتين', few: 'بعد {minutes} دقائق', many: 'بعد {minutes} دقيقة', other: 'بعد {minutes} دقيقة' },
}
const running: Partial<Record<Language, PluralForms>> = {
  'en-US': { one: '{count} reminder running' },
  'fr-FR': { one: '{count} rappel actif' },
  'de-DE': { one: '{count} Erinnerung aktiv' },
  'hi-IN': { one: '{count} रिमाइंडर चल रहा है', other: '{count} रिमाइंडर चल रहे हैं' },
}

export function pluralMessage(language: Language, key: string, params: Record<string, string | number>) {
  const table = key === 'notification.pendingCount' ? pending : key === 'ui.snoozeFor' ? snooze : key === 'app.runningCount' ? running : undefined
  const value = key === 'ui.snoozeFor' ? params.minutes : params.count
  if (!table || typeof value !== 'number')
    return undefined
  const forms = table[language]
  return forms?.[new Intl.PluralRules(language).select(value)] ?? forms?.other
}
