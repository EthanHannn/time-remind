import { describe, expect, it } from 'vitest'
import { mascotStyleOptions, normalizeMascotStyle } from './mascotStyles'
import { normalizeNotificationPalette, resolveNotificationPalette } from './notificationPalette'

describe('notification palettes and 9 companion styles', () => {
  it('supports all 9 mascot styles and pairs auto palette to paper for pets and ink for mecha', () => {
    expect(mascotStyleOptions).toHaveLength(9)
    for (const option of mascotStyleOptions) {
      expect(normalizeMascotStyle(option.id)).toBe(option.id)
      const expectedAuto = option.group === 'mecha' ? 'ink' : 'paper'
      expect(resolveNotificationPalette('auto', option.id)).toBe(expectedAuto)
      expect(resolveNotificationPalette('sage', option.id)).toBe('sage')
      expect(resolveNotificationPalette('paper', option.id)).toBe('paper')
      expect(resolveNotificationPalette('ink', option.id)).toBe('ink')
    }
    expect(normalizeMascotStyle('unknown')).toBe('classic')
    expect(normalizeNotificationPalette('unknown')).toBe('auto')
  })
})
