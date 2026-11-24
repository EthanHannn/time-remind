import type { MascotStyle } from './mascotStyles'
import { getMascotGroup } from './mascotStyles'

export type NotificationPaletteSetting = 'auto' | 'paper' | 'ink' | 'sage'
export type ResolvedNotificationPalette = 'paper' | 'ink' | 'sage'

export const notificationPaletteOptions: readonly NotificationPaletteSetting[] = ['auto', 'paper', 'ink', 'sage']

export function normalizeNotificationPalette(value: unknown): NotificationPaletteSetting {
  if (value === 'paper' || value === 'ink' || value === 'sage')
    return value
  return 'auto'
}

export function resolveNotificationPalette(
  setting: unknown,
  mascotStyle: MascotStyle,
): ResolvedNotificationPalette {
  const normalized = normalizeNotificationPalette(setting)
  if (normalized !== 'auto')
    return normalized
  return getMascotGroup(mascotStyle) === 'mecha' ? 'ink' : 'paper'
}

export function applyPalette(palette: ResolvedNotificationPalette) {
  if (typeof document !== 'undefined')
    document.documentElement.dataset.palette = palette
}
