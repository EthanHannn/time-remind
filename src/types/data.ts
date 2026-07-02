import type { Language } from '../i18n'
import type { NotificationSoundPreset } from '../utils/notificationSound'
import type { Reminder } from './reminder'

export interface ReminderLog {
  id: string
  reminder_id: string
  action: string
  triggered_at: string
  responded_at: string | null
}

export interface FrontendSettings {
  theme: 'light' | 'dark' | 'system'
  language?: Language
  notificationDuration: number
  postponeOptions: number[]
  soundEnabled?: boolean
  soundPreset?: NotificationSoundPreset
  soundVolume?: number
}

export interface ExportData {
  version: string
  exported_at: string
  reminders: Reminder[]
  logs: ReminderLog[]
  settings: Record<string, string>
  frontend_settings?: FrontendSettings
}

export type ImportMode = 'replace' | 'merge'

export interface ImportResult {
  mode: ImportMode
  backup_path: string
  reminders_imported: number
  logs_imported: number
  settings_imported: number
  skipped_logs: number
  message: string
}
