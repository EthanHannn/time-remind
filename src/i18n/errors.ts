type Translate = (key: string, params?: Record<string, string | number>) => string
const codes: Record<string, string> = {
  backupExtension: 'operations.backupExtension',
  backupTooLarge: 'operations.backupTooLarge',
  invalidBackup: 'operations.invalidBackup',
  unsupportedBackupVersion: 'operations.unsupportedBackupVersion',
  invalidSetting: 'operations.invalidSetting',
  invalidReminder: 'operations.invalidReminder',
  unavailable: 'operations.unavailable',
  actionRunning: 'operations.actionRunning',
  actionPending: 'operations.actionPending',
  invalidDate: 'operations.invalidDate',
  dndRange: 'operations.dndRange',
  autostartSyncFailed: 'operations.autostartSyncFailed',
  required: 'form.requiredError',
  interval: 'form.intervalError',
  breakDuration: 'form.breakDurationError',
  actionDuration: 'form.actionDurationError',
}

export function localizedError(error: unknown, t: Translate): string {
  console.error('Application operation failed:', error)
  if (error instanceof DOMException && error.name === 'TimeoutError')
    return t('feedback.loadTimeout')
  if (typeof error === 'string' && error.startsWith('time-remind:')) {
    try {
      const payload = JSON.parse(error.slice('time-remind:'.length))
      if (payload && Object.keys(codes).includes(payload.code)) {
        const params: Record<string, string | number> = {}
        for (const [key, value] of Object.entries(payload.params ?? {})) {
          if (typeof value === 'string' || typeof value === 'number')
            params[key] = value
        }
        if (payload.code === 'invalidSetting' && typeof params.key === 'string') {
          const labels: Record<string, string> = {
            language: 'settings.language',
            theme: 'settings.appearance',
            auto_start: 'settings.autoStart',
            silent_start: 'settings.silentStart',
            notification_duration: 'settings.duration',
            sound_volume: 'settings.soundVolume',
            sound_preset: 'settings.soundPreset',
            sound_enabled: 'settings.sound',
            dnd_enabled: 'settings.dndEnabled',
            dnd_start: 'settings.startTime',
            dnd_end: 'settings.endTime',
            fullscreen_detection_enabled: 'settings.fullscreenDelay',
            postpone_options: 'ui.snoozeOptions',
          }
          params.key = labels[params.key] ? t(labels[params.key]!) : params.key
        }
        return t(codes[payload.code]!, params)
      }
    }
    catch { /* Malformed/unknown IPC errors use the localized summary below. */ }
  }
  return t('operations.unexpected')
}
