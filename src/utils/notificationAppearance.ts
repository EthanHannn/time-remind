export type NotificationAppearance = 'soft' | 'classic'

export function normalizeNotificationAppearance(value: unknown): NotificationAppearance {
  return value === 'classic' ? 'classic' : 'soft'
}
