import type { MascotStyle, PreviewReminderType } from '../utils/mascotStyles'
import type { FrontendSettings } from './data'

export interface NotificationPreviewRequest {
  mascotStyle: MascotStyle
  reminderType: PreviewReminderType
  settings: FrontendSettings
}
