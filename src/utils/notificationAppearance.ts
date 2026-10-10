export type NotificationAppearance = 'soft' | 'minimal' | 'illustrated'

export function normalizeNotificationAppearance(value: unknown): NotificationAppearance {
  if (value === 'classic' || value === 'illustrated')
    return 'illustrated'
  return value === 'minimal' ? 'minimal' : 'soft'
}
