<script setup lang="ts">
import type { Language } from '../i18n'
import type { ExportData, FrontendSettings, ImportMode } from '../types/data'
import type { PlatformCapabilities } from '../types/platform'
import type { MascotStyle } from '../utils/mascotStyles'
import type { NotificationAppearance } from '../utils/notificationAppearance'
import type { NotificationPaletteSetting } from '../utils/notificationPalette'
import type { NotificationSoundPreset } from '../utils/notificationSound'
import { invoke } from '@tauri-apps/api/core'
import { disable as disableAutostart, enable as enableAutostart, isEnabled as isAutostartEnabled } from '@tauri-apps/plugin-autostart'
import { confirm, open, save } from '@tauri-apps/plugin-dialog'
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, useId, useTemplateRef, watch } from 'vue'
import { exportData, importData, readTextFile, writeTextFile } from '../api/data'
import { getPlatformCapabilities } from '../api/platform'
import { useModalFocus } from '../composables/useModalFocus'
import { useTheme } from '../composables/useTheme'
import { loadLanguage, useI18n } from '../i18n'
import { normalizeMascotStyle } from '../utils/mascotStyles'
import { normalizeNotificationAppearance } from '../utils/notificationAppearance'
import { applyPalette, normalizeNotificationPalette, resolveNotificationPalette } from '../utils/notificationPalette'
import { playNotificationSound } from '../utils/notificationSound'
import { appIconMain } from '../utils/reminderVisuals'
import { createSettingsWriter } from '../utils/settingsWriter'
import AppIcon from './AppIcon.vue'
import LanguageSelect from './LanguageSelect.vue'
import MascotStyleSettings from './MascotStyleSettings.vue'
import NotificationAppearanceSettings from './NotificationAppearanceSettings.vue'

const emit = defineEmits<{
  close: []
}>()

const { applyTheme } = useTheme()

const fieldId = useId()
useModalFocus(useTemplateRef<HTMLElement>('dialog'), handleClose)

const AUTO_START_SETTING_KEY = 'auto_start'

const { language: currentLanguage, t, formatError, number } = useI18n()
type SettingsTab = 'appearance' | 'reminders' | 'system'
const activeTab = shallowRef<SettingsTab>('appearance')
const settingsTabs: { id: SettingsTab, labelKey: string }[] = [
  { id: 'appearance', labelKey: 'ui.settingsTabAppearance' },
  { id: 'reminders', labelKey: 'ui.settingsTabReminders' },
  { id: 'system', labelKey: 'ui.settingsTabSystem' },
]
const theme = ref<'light' | 'dark' | 'system'>('system')
const mascotStyle = shallowRef<MascotStyle>('classic')
const notificationAppearance = shallowRef<NotificationAppearance>('soft')
const notificationPalette = shallowRef<NotificationPaletteSetting>('auto')
const language = ref<Language>(currentLanguage.value)
const notificationDuration = ref(30)
const postponeOptions = ref([5, 10, 15])
const soundEnabled = ref(true)
const soundPreset = ref<NotificationSoundPreset>('soft')
const soundVolume = ref(60)
const settingsLoaded = ref(false)
const dndEnabled = ref(false)
const dndStart = ref('22:00')
const dndEnd = ref('08:00')
const autoStart = ref(false)
const autoStartLoading = ref(false)
const silentStart = ref(false)
const silentStartLoading = ref(false)
const fullscreenDetectionEnabled = ref(true)
const exporting = ref(false)
const importing = ref(false)
const message = ref('')
const messageIsError = ref(false)
const saveError = ref('')
const loadFailed = ref(false)
const importMode = ref<ImportMode>('replace')
const platformCapabilities = shallowRef<PlatformCapabilities | null>(null)

const platformCapabilitiesLoaded = computed(() => platformCapabilities.value !== null)
const supportsAutostart = computed(() => platformCapabilities.value?.supportsAutostart ?? false)
const supportsSilentStart = computed(() => platformCapabilities.value?.supportsSilentStart ?? false)
const supportsFullscreenDetection = computed(() => platformCapabilities.value?.supportsFullscreenDetection ?? false)
const supportsLockDetection = computed(() => platformCapabilities.value?.supportsLockDetection ?? false)
const supportsTray = computed(() => platformCapabilities.value?.supportsTray ?? false)
const platformStatusDescription = computed(() => {
  if (!platformCapabilities.value) {
    return t('settings.platformCapabilityLoading')
  }

  if (platformCapabilities.value.isVerifiedReleasePlatform) {
    return t('settings.platformCapabilityVerified')
  }

  return t('settings.platformCapabilityLimited')
})
const autoStartDisabled = computed(() => autoStartLoading.value || !supportsAutostart.value)
const silentStartDisabled = computed(() => silentStartLoading.value || !supportsSilentStart.value)
const fullscreenDetectionDisabled = computed(() => !supportsFullscreenDetection.value)

const importModeDescription = computed(() => {
  return importMode.value === 'replace'
    ? t('settings.replaceDescription')
    : t('settings.mergeDescription')
})

const importRiskLabel = computed(() => {
  return importMode.value === 'replace'
    ? t('settings.replaceRisk')
    : t('settings.mergeRisk')
})

function buildFrontendSettings(): FrontendSettings {
  return {
    theme: theme.value,
    mascotStyle: mascotStyle.value,
    notificationAppearance: notificationAppearance.value,
    notificationPalette: notificationPalette.value,
    language: language.value,
    notificationDuration: notificationDuration.value,
    postponeOptions: postponeOptions.value,
    soundEnabled: soundEnabled.value,
    soundPreset: soundPreset.value,
    soundVolume: soundVolume.value,
  }
}

const settingsWriter = createSettingsWriter(
  values => invoke('save_settings', { settings: values }),
  (error) => { saveError.value = t('feedback.failed', { error: formatError(error) }) },
  (values) => {
    saveError.value = ''
    localStorage.setItem('app-settings', JSON.stringify(buildFrontendSettings()))
    if (values.language)
      void loadLanguage({ language: values.language })
  },
)

async function retrySave() {
  if (loadFailed.value) {
    await initializeSettings()
    return
  }
  try {
    await settingsWriter.flush()
  }
  catch { /* The writer keeps the failed values and reports the error. */ }
}

async function handleClose() {
  if (importing.value || exporting.value)
    return
  try {
    await settingsWriter.flush()
    emit('close')
  }
  catch { /* Keep the settings open so the user can retry. */ }
}

onUnmounted(() => {
  void settingsWriter.flush().catch(() => {})
})

async function loadSettings() {
  const wasLoaded = settingsLoaded.value
  settingsLoaded.value = false
  loadFailed.value = false
  mascotStyle.value = 'classic'
  notificationAppearance.value = 'soft'
  notificationPalette.value = 'auto'
  theme.value = 'system'
  notificationDuration.value = 30
  postponeOptions.value = [5, 10, 15]
  soundEnabled.value = true
  soundPreset.value = 'soft'
  soundVolume.value = 60
  dndEnabled.value = false
  dndStart.value = '22:00'
  dndEnd.value = '08:00'
  autoStart.value = false
  silentStart.value = false
  fullscreenDetectionEnabled.value = true
  let hasFrontendSettingsInDb = false
  try {
    const settings = await invoke<Record<string, string>>('get_all_settings')
    await loadLanguage(settings)
    mascotStyle.value = normalizeMascotStyle(settings.mascot_style)
    notificationAppearance.value = normalizeNotificationAppearance(settings.notification_appearance)
    notificationPalette.value = normalizeNotificationPalette(settings.notification_palette)
    if (settings.notification_appearance)
      hasFrontendSettingsInDb = true
    if (settings.notification_palette)
      hasFrontendSettingsInDb = true
    if (settings.mascot_style)
      hasFrontendSettingsInDb = true
    if (settings.theme) {
      theme.value = settings.theme as 'light' | 'dark' | 'system'
      hasFrontendSettingsInDb = true
    }
    if (settings.language) {
      await loadLanguage(settings)
      language.value = currentLanguage.value
      hasFrontendSettingsInDb = true
    }
    if (settings.notification_duration) {
      notificationDuration.value = Number(settings.notification_duration) || 30
      hasFrontendSettingsInDb = true
    }
    if (settings.postpone_options) {
      postponeOptions.value = JSON.parse(settings.postpone_options) as number[]
      hasFrontendSettingsInDb = true
    }
    if (settings.sound_enabled !== undefined) {
      soundEnabled.value = settings.sound_enabled === 'true'
      hasFrontendSettingsInDb = true
    }
    if (settings.sound_preset) {
      soundPreset.value = normalizeSoundPreset(settings.sound_preset)
      hasFrontendSettingsInDb = true
    }
    if (settings.sound_volume) {
      soundVolume.value = normalizeSoundVolume(Number(settings.sound_volume))
      hasFrontendSettingsInDb = true
    }
    if (settings.dnd_enabled !== undefined) {
      dndEnabled.value = settings.dnd_enabled === 'true'
    }
    if (settings.dnd_start) {
      dndStart.value = settings.dnd_start
    }
    if (settings.dnd_end) {
      dndEnd.value = settings.dnd_end
    }
    if (settings.fullscreen_detection_enabled !== undefined) {
      fullscreenDetectionEnabled.value = settings.fullscreen_detection_enabled === 'true'
    }
    if (settings[AUTO_START_SETTING_KEY] !== undefined) {
      autoStart.value = settings[AUTO_START_SETTING_KEY] === 'true'
    }
    if (settings.silent_start !== undefined) {
      silentStart.value = settings.silent_start === 'true'
    }
  }
  catch (error) {
    saveError.value = t('feedback.failed', { error: formatError(error) })
    loadFailed.value = true
    return false
  }

  const saved = localStorage.getItem('app-settings')
  if (saved && !hasFrontendSettingsInDb) {
    try {
      const settings = JSON.parse(saved) as FrontendSettings
      theme.value = settings.theme || 'system'
      mascotStyle.value = normalizeMascotStyle(settings.mascotStyle)
      notificationAppearance.value = normalizeNotificationAppearance(settings.notificationAppearance)
      notificationPalette.value = normalizeNotificationPalette(settings.notificationPalette)
      language.value = settings.language || 'zh-CN'
      notificationDuration.value = settings.notificationDuration || 30
      postponeOptions.value = settings.postponeOptions || [5, 10, 15]
      soundEnabled.value = settings.soundEnabled ?? true
      soundPreset.value = normalizeSoundPreset(settings.soundPreset || 'soft')
      soundVolume.value = normalizeSoundVolume(settings.soundVolume || 60)
    }
    catch {
      // 忽略旧设置解析失败
    }
  }

  applyTheme(theme.value)
  applyPalette(resolveNotificationPalette(notificationPalette.value, mascotStyle.value))
  language.value = currentLanguage.value
  await nextTick()
  settingsLoaded.value = wasLoaded
  saveError.value = ''
  return true
}

async function loadPlatformCapabilities() {
  try {
    const capabilities = await getPlatformCapabilities()
    platformCapabilities.value = capabilities

    if (!capabilities.supportsAutostart) {
      autoStart.value = false
      silentStart.value = false
      await invoke('save_setting', { key: AUTO_START_SETTING_KEY, value: 'false' })
      await invoke('save_setting', { key: 'silent_start', value: 'false' })
    }

    if (!capabilities.supportsFullscreenDetection) {
      fullscreenDetectionEnabled.value = false
      await invoke('save_setting', { key: 'fullscreen_detection_enabled', value: 'false' })
    }
  }
  catch (err) {
    platformCapabilities.value = {
      platform: 'unknown',
      isVerifiedReleasePlatform: false,
      supportsFullscreenDetection: false,
      supportsLockDetection: false,
      supportsTray: false,
      supportsAutostart: false,
      supportsSilentStart: false,
    }
    autoStart.value = false
    silentStart.value = false
    fullscreenDetectionEnabled.value = false
    await Promise.allSettled([
      invoke('save_setting', { key: AUTO_START_SETTING_KEY, value: 'false' }),
      invoke('save_setting', { key: 'silent_start', value: 'false' }),
      invoke('save_setting', { key: 'fullscreen_detection_enabled', value: 'false' }),
    ])
    messageIsError.value = true
    message.value = t('settings.platformCapabilityLoadFailed', { error: formatError(err) })
    console.error('Failed to load platform capabilities:', err)
  }
}

async function loadAutoStart() {
  if (!supportsAutostart.value) {
    autoStart.value = false
    return
  }

  autoStartLoading.value = true
  try {
    const systemEnabled = await isAutostartEnabled()

    if (systemEnabled) {
      autoStart.value = true
      await invoke('save_setting', { key: AUTO_START_SETTING_KEY, value: 'true' })
      return
    }

    if (autoStart.value) {
      await enableAutostart()
      autoStart.value = await isAutostartEnabled()
      await invoke('save_setting', { key: AUTO_START_SETTING_KEY, value: String(autoStart.value) })
      return
    }

    autoStart.value = false
  }
  catch (err) {
    messageIsError.value = true
    message.value = t('settings.loadAutoStartFailed', { error: formatError(err) })
    console.error('Failed to load autostart status:', err)
  }
  finally {
    autoStartLoading.value = false
  }
}

async function initializeSettings() {
  if (!await loadSettings())
    return
  await loadPlatformCapabilities()
  await loadAutoStart()
  settingsLoaded.value = true
}

onMounted(initializeSettings)

watch([theme, mascotStyle, notificationAppearance, notificationPalette, language, notificationDuration, postponeOptions, soundEnabled, soundPreset, soundVolume], () => {
  if (!settingsLoaded.value)
    return
  applyTheme(theme.value)
  applyPalette(resolveNotificationPalette(notificationPalette.value, mascotStyle.value))
  settingsWriter.enqueue({
    notification_appearance: notificationAppearance.value,
    notification_palette: notificationPalette.value,
    mascot_style: mascotStyle.value,
    theme: theme.value,
    language: language.value,
    notification_duration: String(notificationDuration.value),
    postpone_options: JSON.stringify(postponeOptions.value),
    sound_enabled: String(soundEnabled.value),
    sound_preset: soundPreset.value,
    sound_volume: String(soundVolume.value),
  })
}, { deep: true })

function normalizeSoundPreset(value: string): NotificationSoundPreset {
  return value === 'bright' || value === 'calm' || value === 'anime' || value === 'arcade' ? value : 'soft'
}

function normalizeSoundVolume(value: number): number {
  return Math.min(Math.max(Number.isFinite(value) ? Math.round(value) : 60, 0), 100)
}

function testNotificationSound() {
  playNotificationSound({
    preset: soundPreset.value,
    volume: soundVolume.value,
  })
}

watch(soundPreset, () => {
  if (!settingsLoaded.value || !soundEnabled.value) {
    return
  }

  testNotificationSound()
})

watch([dndEnabled, dndStart, dndEnd], () => {
  if (!settingsLoaded.value)
    return
  settingsWriter.enqueue({ dnd_enabled: String(dndEnabled.value), dnd_start: dndStart.value, dnd_end: dndEnd.value })
})

watch(fullscreenDetectionEnabled, () => {
  if (!settingsLoaded.value || !supportsFullscreenDetection.value)
    return
  settingsWriter.enqueue({ fullscreen_detection_enabled: String(fullscreenDetectionEnabled.value) })
})

async function handleAutoStartChange() {
  if (!supportsAutostart.value) {
    autoStart.value = false
    await invoke('save_setting', { key: AUTO_START_SETTING_KEY, value: 'false' })
    return
  }

  autoStartLoading.value = true
  const previousAutoStart = !autoStart.value
  const previousSilentStart = silentStart.value
  try {
    if (autoStart.value) {
      await enableAutostart()
      await invoke('save_setting', { key: AUTO_START_SETTING_KEY, value: 'true' })
      await invoke('save_setting', { key: 'silent_start', value: String(silentStart.value) })
    }
    else {
      await disableAutostart()
      silentStart.value = false
      await invoke('save_setting', { key: AUTO_START_SETTING_KEY, value: 'false' })
      await invoke('save_setting', { key: 'silent_start', value: 'false' })
    }

    autoStart.value = await isAutostartEnabled()
    await invoke('save_setting', { key: AUTO_START_SETTING_KEY, value: String(autoStart.value) })
  }
  catch (err) {
    autoStart.value = previousAutoStart
    silentStart.value = previousSilentStart
    messageIsError.value = true
    message.value = t('settings.autoStartFailed', { error: formatError(err) })
    console.error('Failed to update autostart:', err)
  }
  finally {
    autoStartLoading.value = false
  }
}

async function handleSilentStartChange() {
  if (!supportsSilentStart.value) {
    silentStart.value = false
    return
  }

  silentStartLoading.value = true
  const previousValue = !silentStart.value
  try {
    // 刷新自启注册表项（确保包含 --autostart 参数），enable 是幂等操作
    if (autoStart.value) {
      await enableAutostart()
      autoStart.value = await isAutostartEnabled()
    }

    await invoke('save_setting', { key: 'silent_start', value: String(silentStart.value) })
  }
  catch (err) {
    silentStart.value = previousValue
    messageIsError.value = true
    message.value = t('settings.silentStartFailed', { error: formatError(err) })
    console.error('Failed to update silent start:', err)
  }
  finally {
    silentStartLoading.value = false
  }
}

async function handleExport() {
  if (exporting.value || importing.value)
    return
  exporting.value = true
  message.value = ''
  messageIsError.value = false
  try {
    await settingsWriter.flush()
    const data = await exportData()
    const json = JSON.stringify(data, null, 2)

    const filePath = await save({
      defaultPath: 'time-remind-backup.json',
      filters: [
        { name: 'JSON', extensions: ['json'] },
      ],
    })

    if (filePath) {
      await writeTextFile(filePath, json)
      message.value = t('settings.exportSuccess')
    }
  }
  catch (err) {
    messageIsError.value = true
    message.value = t('settings.exportFailed', { error: formatError(err) })
    console.error('Export failed:', err)
  }
  finally {
    exporting.value = false
  }
}

async function handleImport() {
  if (importing.value || exporting.value)
    return
  importing.value = true
  message.value = ''
  messageIsError.value = false
  try {
    await settingsWriter.flush()
    const confirmed = await confirm(
      importMode.value === 'replace'
        ? t('settings.importConfirmReplace')
        : t('settings.importConfirmMerge'),
      {
        title: t('settings.importConfirmTitle'),
        okLabel: t('settings.importConfirmOk'),
        cancelLabel: t('common.cancel'),
      },
    )

    if (!confirmed) {
      return
    }

    const filePath = await open({
      multiple: false,
      filters: [
        { name: 'JSON', extensions: ['json'] },
      ],
    })

    if (filePath) {
      const content = await readTextFile(filePath)
      const data = JSON.parse(content) as ExportData
      settingsLoaded.value = false
      const result = await importData(data, importMode.value)
      localStorage.removeItem('app-settings')
      if (importMode.value === 'replace')
        localStorage.removeItem('app-language')
      await loadSettings()
      messageIsError.value = result.autostart_sync_failed
      message.value = t('operations.importSummary', {
        mode: t(result.mode === 'replace' ? 'settings.replaceImport' : 'settings.mergeImport'),
        reminders: result.reminders_imported,
        logs: result.logs_imported,
        settings: result.settings_imported,
        skipped: result.skipped_logs,
        path: `\u2068${result.backup_path}\u2069`,
      }) + (result.autostart_sync_failed ? ` ${t('operations.autostartSyncFailed')}` : '')
    }
  }
  catch (err) {
    messageIsError.value = true
    message.value = t('settings.importFailed', { error: err instanceof SyntaxError ? t('operations.invalidBackup') : formatError(err) })
    console.error('Import failed:', err)
  }
  finally {
    settingsLoaded.value = !loadFailed.value
    importing.value = false
  }
}
</script>

<template>
  <div class="settings-overlay" @click.self="handleClose">
    <div ref="dialog" class="settings-container" role="dialog" aria-modal="true" :aria-labelledby="`${fieldId}-title`" tabindex="-1">
      <div class="settings-header">
        <div class="header-copy">
          <div class="brand-row">
            <img :src="appIconMain" alt="Time Remind" class="brand-icon">
            <div>
              <h2 :id="`${fieldId}-title`" class="settings-title">
                {{ t('settings.title') }}
              </h2>
              <p class="settings-subtitle">
                {{ t('settings.subtitle') }}
              </p>
            </div>
          </div>
        </div>

        <button class="close-button" :aria-label="t('common.close')" type="button" @click="handleClose">
          <AppIcon name="close" />
        </button>
      </div>

      <div class="settings-tabs-bar">
        <div class="settings-tabs" role="tablist" :aria-label="t('settings.title')">
          <button
            v-for="tab in settingsTabs"
            :id="`${fieldId}-tab-${tab.id}`"
            :key="tab.id"
            type="button"
            role="tab"
            class="settings-tab"
            :class="{ 'settings-tab-active': activeTab === tab.id }"
            :aria-selected="activeTab === tab.id"
            :aria-controls="`${fieldId}-panel-${tab.id}`"
            @click="activeTab = tab.id"
          >
            {{ t(tab.labelKey) }}
          </button>
        </div>
      </div>

      <p v-if="saveError" role="alert" class="message message-error save-error-banner">
        {{ saveError }} <button type="button" @click="retrySave">
          {{ t('feedback.retry') }}
        </button>
      </p>

      <div class="settings-content" :inert="importing || exporting || !settingsLoaded">
        <!-- Tab 1: Appearance & Companion -->
        <div
          v-show="activeTab === 'appearance'"
          :id="`${fieldId}-panel-appearance`"
          role="tabpanel"
          :aria-labelledby="`${fieldId}-tab-appearance`"
          class="settings-panel"
        >
          <section class="setting-section appearance-section">
            <div class="appearance-row">
              <h3 class="section-title">
                {{ t('settings.appearance') }}
              </h3>
              <div class="theme-selector" role="group" :aria-label="t('settings.appearance')">
                <button v-for="option in ['light', 'dark', 'system'] as const" :key="option" class="theme-button" :class="{ 'theme-button-active': theme === option }" :aria-pressed="theme === option" :aria-label="t(option === 'system' ? 'settings.systemTheme' : `settings.${option}`)" :title="t(option === 'system' ? 'settings.systemTheme' : `settings.${option}`)" type="button" @click="theme = option">
                  <svg class="theme-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <template v-if="option === 'light'"><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5" /></template>
                    <path v-else-if="option === 'dark'" d="M20 14A8 8 0 0 1 10 4a8.5 8.5 0 1 0 10 10Z" />
                    <template v-else><rect x="3" y="4" width="18" height="13" rx="2" /><path d="M8 21h8m-4-4v4" /></template>
                  </svg>
                  <span class="theme-label">{{ t(option === 'system' ? 'settings.systemTheme' : `settings.${option}`) }}</span>
                </button>
              </div>
            </div>
          </section>

          <section class="setting-section visual-settings">
            <NotificationAppearanceSettings v-model="notificationAppearance" v-model:palette="notificationPalette" :disabled="!settingsLoaded" />
            <MascotStyleSettings v-model="mascotStyle" :settings="buildFrontendSettings()" :disabled="!settingsLoaded" />
          </section>
        </div>

        <!-- Tab 2: Reminders & Sound -->
        <div
          v-show="activeTab === 'reminders'"
          :id="`${fieldId}-panel-reminders`"
          role="tabpanel"
          :aria-labelledby="`${fieldId}-tab-reminders`"
          class="settings-panel"
        >
          <section class="setting-section">
            <div class="section-heading">
              <h3 class="section-title">
                {{ t('settings.notification') }}
              </h3>
            </div>

            <div class="setting-row">
              <label class="setting-label" :for="`${fieldId}-duration`">{{ t('settings.duration') }}</label>
              <input :id="`${fieldId}-duration`" v-model.number="notificationDuration" class="setting-input" type="number" min="5" max="120">
            </div>

            <div class="setting-row sound-row">
              <label class="setting-label">{{ t('settings.sound') }}</label>
              <label class="switch">
                <input v-model="soundEnabled" :aria-label="t('settings.sound')" type="checkbox">
                <span class="slider" />
              </label>
            </div>

            <Transition name="slide-down">
              <div v-if="soundEnabled" class="sound-panel">
                <div class="setting-row">
                  <label class="setting-label">{{ t('settings.soundPreset') }}</label>
                  <select v-model="soundPreset" :aria-label="t('settings.soundPreset')" class="setting-input">
                    <option value="soft">
                      {{ t('settings.soundSoft') }}
                    </option>
                    <option value="bright">
                      {{ t('settings.soundBright') }}
                    </option>
                    <option value="calm">
                      {{ t('settings.soundCalm') }}
                    </option>
                    <option value="anime">
                      {{ t('settings.soundAnime') }}
                    </option>
                    <option value="arcade">
                      {{ t('settings.soundArcade') }}
                    </option>
                  </select>
                  <button class="data-button sound-test-button" type="button" @click="testNotificationSound">
                    {{ t('settings.testSound') }}
                  </button>
                </div>

                <div class="setting-row">
                  <label class="setting-label">{{ t('settings.soundVolume') }}</label>
                  <input v-model.number="soundVolume" :aria-label="t('settings.soundVolume')" class="sound-range" type="range" min="0" max="100">
                  <span class="sound-volume">{{ number(soundVolume / 100, { style: 'percent' }) }}</span>
                </div>
              </div>
            </Transition>
          </section>

          <section class="setting-section">
            <div class="section-heading">
              <h3 class="section-title">
                {{ t('settings.dnd') }}
              </h3>
            </div>

            <div class="setting-row">
              <label class="setting-label">{{ t('settings.dndEnabled') }}</label>
              <label class="switch">
                <input v-model="dndEnabled" :aria-label="t('settings.dndEnabled')" type="checkbox">
                <span class="slider" />
              </label>
            </div>

            <Transition name="slide-down">
              <div v-if="dndEnabled" class="time-range">
                <div class="time-field">
                  <label class="time-label">{{ t('settings.startTime') }}</label>
                  <input v-model="dndStart" :aria-label="t('settings.startTime')" class="setting-input" type="time">
                </div>
                <div class="time-field">
                  <label class="time-label">{{ t('settings.endTime') }}</label>
                  <input v-model="dndEnd" :aria-label="t('settings.endTime')" class="setting-input" type="time">
                </div>
              </div>
            </Transition>
          </section>
        </div>

        <!-- Tab 3: System & Data -->
        <div
          v-show="activeTab === 'system'"
          :id="`${fieldId}-panel-system`"
          role="tabpanel"
          :aria-labelledby="`${fieldId}-tab-system`"
          class="settings-panel"
        >
          <section class="setting-section">
            <div class="section-heading">
              <h3 class="section-title">
                {{ t('settings.system') }}
              </h3>
            </div>

            <div class="setting-row language-row">
              <label class="setting-label" :for="`${fieldId}-language`">{{ t('settings.language') }}</label>
              <LanguageSelect :id="`${fieldId}-language`" v-model="language" class="language-select" />
            </div>

            <div class="setting-row setting-card">
              <div class="setting-card-main">
                <div class="setting-card-header">
                  <div class="setting-card-copy">
                    <label class="setting-label">{{ t('settings.autoStart') }}</label>
                    <p v-if="platformCapabilitiesLoaded && !supportsAutostart" class="setting-description setting-warning">
                      {{ t('settings.unsupportedOnPlatform') }}
                    </p>
                  </div>
                  <label class="switch" :class="{ 'switch-disabled': autoStartDisabled, 'switch-loading': autoStartLoading }">
                    <input
                      v-model="autoStart" :aria-label="t('settings.autoStart')"
                      :disabled="autoStartDisabled"
                      type="checkbox"
                      @change="handleAutoStartChange"
                    >
                    <span class="slider" />
                  </label>
                </div>

                <Transition name="slide-down">
                  <div v-if="autoStart" class="setting-subcard">
                    <div class="setting-subcard-copy">
                      <label class="setting-label">{{ t('settings.silentStart') }}</label>
                      <p class="setting-description">
                        {{ t('settings.silentStartDescription') }}
                      </p>
                      <p v-if="platformCapabilitiesLoaded && !supportsSilentStart" class="setting-description setting-warning">
                        {{ t('settings.unsupportedOnPlatform') }}
                      </p>
                    </div>
                    <label class="switch" :class="{ 'switch-disabled': silentStartDisabled, 'switch-loading': silentStartLoading }">
                      <input
                        v-model="silentStart" :aria-label="t('settings.silentStart')"
                        :disabled="silentStartDisabled"
                        type="checkbox"
                        @change="handleSilentStartChange"
                      >
                      <span class="slider" />
                    </label>
                  </div>
                </Transition>
              </div>
            </div>

            <div class="setting-row">
              <div class="setting-inline-copy">
                <label class="setting-label">{{ t('settings.fullscreenDelay') }}</label>
                <p v-if="platformCapabilitiesLoaded && !supportsFullscreenDetection" class="setting-description setting-warning">
                  {{ t('settings.unsupportedOnPlatform') }}
                </p>
              </div>
              <label class="switch" :class="{ 'switch-disabled': fullscreenDetectionDisabled }">
                <input v-model="fullscreenDetectionEnabled" :aria-label="t('settings.fullscreenDelay')" :disabled="fullscreenDetectionDisabled" type="checkbox">
                <span class="slider" />
              </label>
            </div>
            <details class="platform-capability-card">
              <summary>{{ t('settings.platformCapabilities') }}</summary>
              <div class="setting-card-main">
                <div class="setting-card-copy">
                  <p class="setting-description">
                    {{ platformStatusDescription }}
                  </p>
                  <p v-if="platformCapabilitiesLoaded && !supportsLockDetection" class="setting-description setting-warning">
                    {{ t('settings.lockDetectionUnsupported') }}
                  </p>
                  <p v-if="platformCapabilitiesLoaded && !supportsTray" class="setting-description setting-warning">
                    {{ t('settings.trayUnsupported') }}
                  </p>
                </div>
              </div>
            </details>
          </section>

          <section class="setting-section">
            <div class="section-heading">
              <h3 class="section-title">
                {{ t('settings.dataManagement') }}
              </h3>
            </div>

            <div class="import-mode">
              <button
                class="mode-button"
                :class="{ 'mode-button-active': importMode === 'replace' }"
                :disabled="exporting || importing"
                type="button"
                @click="importMode = 'replace'"
              >
                {{ t('settings.replaceImport') }}
              </button>
              <button
                class="mode-button"
                :class="{ 'mode-button-active': importMode === 'merge' }"
                :disabled="exporting || importing"
                type="button"
                @click="importMode = 'merge'"
              >
                {{ t('settings.mergeImport') }}
              </button>
            </div>

            <div class="data-actions">
              <button class="data-button" :disabled="exporting" type="button" @click="handleExport">
                <span>{{ exporting ? t('settings.exporting') : t('settings.exportData') }}</span>
              </button>
              <button class="data-button" :disabled="importing" type="button" @click="handleImport">
                <span>{{ importing ? t('settings.importing') : t('settings.importData') }}</span>
              </button>
            </div>

            <p class="hint">
              {{ importModeDescription }}
            </p>
            <p class="hint">
              {{ importRiskLabel }}
            </p>
            <p class="hint">
              {{ t('settings.exportHint') }}
            </p>
            <p v-if="message" class="message" :class="{ 'message-error': messageIsError }" :role="messageIsError ? 'alert' : 'status'">
              {{ message }}
            </p>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 12px;
  background: rgb(14 16 20 / 42%);
  backdrop-filter: blur(8px);
}

.settings-container {
  --settings-surface: var(--card-surface);
  --settings-inset: var(--bg-secondary);
  --settings-border: var(--border-color);
  --settings-control: var(--bg-primary);
  --settings-selected: var(--color-primary-light);
  width: min(100%, 620px);
  max-height: min(860px, calc(100dvh - 24px));
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-radius: 20px;
  border: 1px solid var(--settings-border);
  background: var(--app-shell-bg);
  box-shadow: var(--shadow-md);
  container-type: inline-size;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 18px 12px;
  background: transparent;
  flex-shrink: 0;
}

.brand-row { display: flex; align-items: center; gap: 10px; }
.brand-icon { width: 30px; height: 30px; flex-shrink: 0; }
.settings-title { margin: 0; font-size: 17px; font-weight: 700; color: var(--text-primary); }
.settings-subtitle { margin: 2px 0 0; font-size: 11px; line-height: 1.4; color: var(--text-secondary); }
.close-button { width: 30px; height: 30px; flex-shrink: 0; display: grid; place-items: center; border-radius: 9px; color: var(--text-secondary); background: var(--settings-inset); }
.close-button:hover { color: var(--text-primary); background: var(--bg-tertiary); }

.settings-tabs-bar {
  padding: 0 18px 12px;
  border-bottom: 1px solid var(--settings-border);
  flex-shrink: 0;
}

.settings-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 4px;
  padding: 3px;
  border-radius: 12px;
  background: var(--settings-inset);
  border: 1px solid var(--settings-border);
}

.settings-tab {
  padding: 7px 10px;
  border-radius: 9px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
  transition: background-color 140ms ease, color 140ms ease, box-shadow 140ms ease;
}

.settings-tab:hover {
  color: var(--text-primary);
}

.settings-tab-active {
  background: var(--bg-primary);
  color: var(--text-primary);
  font-weight: 600;
  box-shadow: var(--shadow-sm);
}

.save-error-banner {
  margin: 10px 18px 0;
}

.settings-content {
  padding: 14px 18px 18px;
  overflow-y: auto;
  overscroll-behavior: contain;
  min-height: 0;
  scrollbar-gutter: stable;
}

.settings-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.setting-section {
  min-width: 0;
  padding: 14px;
  border: 1px solid var(--settings-border);
  border-radius: 14px;
  background: var(--settings-surface);
}
.section-heading { margin-bottom: 8px; }
.section-title { margin: 0; font-size: 13px; font-weight: 650; color: var(--text-primary); }

.appearance-section { display: grid; gap: 10px; }
.appearance-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-width: 0; }
.theme-selector { display: flex; padding: 3px; border-radius: 10px; gap: 3px; background: var(--settings-inset); border: 1px solid var(--settings-border); flex-shrink: 0; }
.theme-button { display: flex; align-items: center; justify-content: center; gap: 6px; padding: 6px 9px; border-radius: 7px; color: var(--text-secondary); font-size: 12px; min-width: 36px; transition: background-color 140ms ease, color 140ms ease; }
.theme-icon { width: 16px; height: 16px; flex-shrink: 0; }
.theme-button-active { color: var(--text-primary); background: var(--bg-primary); font-weight: 600; box-shadow: var(--shadow-sm); }
.theme-button:hover { color: var(--text-primary); }
.language-select { width: 180px; max-width: 65%; }

.setting-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 11px 0; min-width: 0; }
.setting-row + .setting-row { border-top: 1px solid var(--settings-border); }
.setting-label, .time-label { font-size: 12px; line-height: 1.5; color: var(--text-primary); }
.setting-label { min-width: 0; overflow-wrap: anywhere; }
.setting-card-main { width: 100%; min-width: 0; }
.setting-card-header, .setting-subcard { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.setting-card-copy, .setting-subcard-copy, .setting-inline-copy { min-width: 0; }
.setting-subcard { margin-top: 10px; padding: 10px; border-radius: 10px; background: var(--settings-inset); }
.setting-description { margin: 4px 0 0; font-size: 11px; line-height: 1.5; color: var(--text-secondary); overflow-wrap: anywhere; }
.setting-warning { color: var(--color-warning); }
.platform-capability-card { border-top: 1px solid var(--settings-border); padding-top: 10px; margin-top: 4px; color: var(--text-secondary); }
.platform-capability-card summary { cursor: pointer; font-size: 11px; }
.platform-capability-card[open] summary { margin-bottom: 7px; }

.setting-input { min-width: 0; padding: 7px 9px; border: 1px solid var(--settings-border); border-radius: 8px; background: var(--settings-control); color: var(--text-primary); font-size: 12px; line-height: 1.5; }
.setting-row > .setting-input { max-width: 55%; }
.setting-input[type='number'] { width: 72px; flex-shrink: 0; }
.switch { position: relative; display: inline-block; width: 36px; height: 22px; flex: 0 0 36px; }
.switch-disabled { opacity: 0.5; cursor: not-allowed; }
.switch-loading { cursor: wait; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider { position: absolute; inset: 0; border-radius: 999px; background: var(--bg-tertiary); border: 1px solid var(--settings-border); transition: background-color 150ms ease; }
.slider::before { content: ''; position: absolute; width: 16px; height: 16px; left: 2px; top: 2px; border-radius: 50%; background: var(--switch-thumb); box-shadow: 0 1px 3px rgb(0 0 0 / 18%); transition: transform 150ms ease; }
input:checked + .slider { background: var(--color-primary); border-color: transparent; }
input:checked + .slider::before { transform: translateX(14px); }
.switch:has(:focus-visible) { outline: 2px solid var(--color-primary); outline-offset: 3px; border-radius: 999px; }

.sound-panel { padding: 0 10px; border-radius: 10px; background: var(--settings-inset); }
.sound-panel .setting-row { flex-wrap: wrap; }
.sound-panel .setting-row > .setting-label { flex: 1 0 60px; }
.sound-panel .setting-input { max-width: 55%; flex: 1; }
.sound-range { flex: 1 1 90px; min-width: 60px; width: 90px; accent-color: var(--color-primary); }
.sound-volume { font-size: 11px; font-variant-numeric: tabular-nums; color: var(--text-secondary); min-width: 32px; text-align: right; }
.sound-test-button { flex: 0 1 auto; }
.time-range { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; margin-top: 4px; }
.time-field { display: flex; flex-direction: column; gap: 5px; min-width: 0; }
.time-field .setting-input { width: 100%; }

.import-mode { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 4px; background: var(--settings-inset); padding: 3px; border-radius: 10px; border: 1px solid var(--settings-border); }
.mode-button { padding: 7px 8px; font-size: 12px; color: var(--text-secondary); border-radius: 7px; transition: background-color 140ms ease, color 140ms ease; }
.mode-button-active { background: var(--bg-primary); color: var(--text-primary); font-weight: 600; box-shadow: var(--shadow-sm); }
.data-actions { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; margin-top: 10px; }
.data-button { padding: 8px 10px; border-radius: 9px; font-size: 12px; line-height: 1.5; color: var(--text-primary); border: 1px solid var(--settings-border); background: var(--bg-primary); transition: border-color 140ms ease, color 140ms ease; }
.data-button:hover:not(:disabled), .mode-button:hover:not(:disabled) { border-color: var(--color-primary); color: var(--text-primary); }
.data-button:disabled, .mode-button:disabled { opacity: 0.5; cursor: wait; }
.hint { margin: 8px 0 0; font-size: 11px; line-height: 1.5; color: var(--text-secondary); }
.message { margin: 10px 0 0; padding: 10px; border-radius: 10px; background: var(--color-primary-light); color: var(--text-primary); font-size: 12px; }
.message-error { color: var(--color-danger); }

button:focus-visible, select:focus-visible, input:focus-visible, summary:focus-visible { outline: 2px solid var(--color-primary); outline-offset: 2px; }
.slide-down-enter-active, .slide-down-leave-active { transition: opacity 150ms ease; }
.slide-down-enter-from, .slide-down-leave-to { opacity: 0; }

@container (max-width: 520px) {
  .theme-label { display: none; }
  .settings-header { padding: 12px 14px 10px; }
  .settings-tabs-bar { padding: 0 14px 10px; }
  .settings-content { padding: 12px 14px 14px; }
  .setting-section { padding: 12px; }
}

@media (prefers-reduced-motion: reduce) {
  .slider, .slider::before, .settings-tab, .theme-button, .mode-button, .data-button, .slide-down-enter-active, .slide-down-leave-active { transition: none; }
}
</style>
