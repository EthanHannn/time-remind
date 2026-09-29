import { describe, expect, it } from 'vitest'
import { messages } from './messages'

interface MessageMap {
  readonly [key: string]: MessageNode
}

type MessageNode = string | MessageMap

function flattenKeys(node: MessageNode, prefix = ''): string[] {
  if (typeof node === 'string')
    return [prefix]

  return Object.entries(node).flatMap(([key, value]) => {
    const nextPrefix = prefix ? `${prefix}.${key}` : key
    return flattenKeys(value, nextPrefix)
  })
}

function flattenMessages(node: MessageMap, prefix = ''): Record<string, string> {
  return Object.fromEntries(Object.entries(node).flatMap(([key, value]) => {
    const path = prefix ? `${prefix}.${key}` : key
    return typeof value === 'string' ? [[path, value]] : Object.entries(flattenMessages(value, path))
  }))
}

describe('messages', () => {
  it('keeps every language aligned with the English message schema', () => {
    const expectedKeys = flattenKeys(messages['en-US']).sort()

    for (const [language, message] of Object.entries(messages)) {
      expect(flattenKeys(message).sort(), language).toEqual(expectedKeys)
    }
  })

  it('preserves interpolation placeholders and has no empty translations', () => {
    const english = flattenMessages(messages['en-US'])
    const placeholders = (value: string) => [...value.matchAll(/\{\w+\}/g)].map(match => match[0]).sort()
    for (const [language, message] of Object.entries(messages)) {
      for (const [key, value] of Object.entries(flattenMessages(message))) {
        expect(value.trim(), `${language}.${key}`).not.toBe('')
        expect(placeholders(value), `${language}.${key}`).toEqual(placeholders(english[key]!))
      }
    }
  })

  it('does not silently reuse English copy except shared names and loanwords', () => {
    const shared = ['common.appName', 'reminderTypes.custom.iconText']
    const loanwords: Record<string, string[]> = {
      'fr-FR': ['common.minute', 'common.minutes', 'app.stats', 'form.type', 'form.content', 'settings.notification', 'settings.soundVolume', 'stats.title'],
      'de-DE': ['reminder.countdown', 'form.name', 'settings.systemTheme', 'settings.system', 'stats.trend'],
      'es-ES': ['common.minute', 'common.minutes'],
      'pt-BR': ['common.minute', 'common.minutes', 'settings.soundVolume'],
      'it-IT': ['common.minute', 'common.minutes', 'settings.soundVolume'],
      'id-ID': ['settings.soundVolume'],
      'ms-MY': ['common.minute', 'common.minutes', 'common.durationUnit', 'settings.importData', 'stats.trend'],
    }
    const english = flattenMessages(messages['en-US'])
    for (const [language, message] of Object.entries(messages)) {
      if (language === 'en-US')
        continue
      const allowed = new Set([...shared, ...loanwords[language] ?? []])
      for (const [key, value] of Object.entries(flattenMessages(message))) {
        if (!allowed.has(key))
          expect(value, `${language}.${key}`).not.toBe(english[key])
      }
    }
  })
})
