import { expect, it } from 'vitest'
import { normalizeNotificationAppearance } from './notificationAppearance'

it('keeps existing soft settings and upgrades legacy classic to the illustrated layout', () => {
  expect(normalizeNotificationAppearance('soft')).toBe('soft')
  expect(normalizeNotificationAppearance('classic')).toBe('illustrated')
  expect(normalizeNotificationAppearance('minimal')).toBe('minimal')
  expect(normalizeNotificationAppearance('illustrated')).toBe('illustrated')
  expect(normalizeNotificationAppearance(undefined)).toBe('soft')
  expect(normalizeNotificationAppearance('invalid-backup-value')).toBe('soft')
})
