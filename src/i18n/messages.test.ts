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

describe('messages', () => {
  it('keeps every language aligned with the English message schema', () => {
    const expectedKeys = flattenKeys(messages['en-US']).sort()

    for (const [language, message] of Object.entries(messages)) {
      expect(flattenKeys(message).sort(), language).toEqual(expectedKeys)
    }
  })
})
