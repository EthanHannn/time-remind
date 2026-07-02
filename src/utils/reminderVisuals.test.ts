import { describe, expect, it } from 'vitest'
import { getReminderVisual, reminderTypeOptions } from './reminderVisuals'

describe('reminderVisuals', () => {
  it('returns stable preset options in display order', () => {
    expect(reminderTypeOptions.map(option => option.type)).toEqual([
      'drink',
      'rest',
      'eye_care',
      'custom',
    ])
  })

  it('falls back to custom visual for unknown types', () => {
    expect(getReminderVisual('unknown').type).toBe('custom')
  })

  it('keeps rest reminder defaults aligned with break countdown flow', () => {
    const rest = getReminderVisual('rest')

    expect(rest.defaultIntervalMinutes).toBe(60)
    expect(rest.defaultBreakDurationMinutes).toBe(5)
    expect(rest.defaultBreakNotificationEnabled).toBe(true)
    expect(rest.defaultActionDurationSeconds).toBe(300)
  })

  it('keeps eye care reminder defaults aligned with 20-20-20 flow', () => {
    const eyeCare = getReminderVisual('eye_care')

    expect(eyeCare.defaultIntervalMinutes).toBe(20)
    expect(eyeCare.defaultActionDurationSeconds).toBe(20)
  })
})
