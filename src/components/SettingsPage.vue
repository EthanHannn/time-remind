<script setup lang="ts">
import type { Language } from '../i18n'
import type { ExportData, FrontendSettings, ImportMode } from '../types/data'
import type { PlatformCapabilities } from '../types/platform'
import type { NotificationSoundPreset } from '../utils/notificationSound'
import { invoke } from '@tauri-apps/api/core'
import { disable as disableAutostart, enable as enableAutostart, isEnabled as isAutostartEnabled } from '@tauri-apps/plugin-autostart'
import { confirm, open, save } from '@tauri-apps/plugin-dialog'
import { computed, onMounted, ref, shallowRef, watch } from 'vue'
import { exportData, importData, readTextFile, writeTextFile } from '../api/data'
import { getPlatformCapabilities } from '../api/platform'
import { loadLanguage, useI18n } from '../i18n'
import { playNotificationSound } from '../utils/notificationSound'
import { appIconMain } from '../utils/reminderVisuals'

const emit = defineEmits<{
  close: []
}>()

const AUTO_START_SETTING_KEY = 'auto_start'

const { language: currentLanguage, languageOptions, setLanguage, t } = useI18n()
const theme = ref<'light' | 'dark' | 'system'>('system')
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
    language: language.value,
    notificationDuration: notificationDuration.value,
    postponeOptions: postponeOptions.value,
    soundEnabled: soundEnabled.value,
    soundPreset: soundPreset.value,
    soundVolume: soundVolume.value,
  }
}

async function saveFrontendSettings() {
  await invoke('save_setting', { key: 'theme', value: theme.value })
  await invoke('save_setting', { key: 'language', value: language.value })
  await invoke('save_setting', { key: 'notification_duration', value: String(notificationDuration.value) })
  await invoke('save_setting', { key: 'postpone_options', value: JSON.stringify(postponeOptions.value) })
  await invoke('save_setting', { key: 'sound_enabled', value: String(soundEnabled.value) })
  await invoke('save_setting', { key: 'sound_preset', value: soundPreset.value })
  await invoke('save_setting', { key: 'sound_volume', value: String(soundVolume.value) })
}

async function loadSettings() {
  let hasFrontendSettingsInDb = false
  try {
    const settings = await invoke<Record<string, string>>('get_all_settings')
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
  catch {
    // 忽略数据库读取失败
  }

  const saved = localStorage.getItem('app-settings')
  if (saved && !hasFrontendSettingsInDb) {
    try {
      const settings = JSON.parse(saved) as FrontendSettings
      theme.value = settings.theme || 'system'
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
  language.value = currentLanguage.value
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
    message.value = t('settings.platformCapabilityLoadFailed', { error: String(err) })
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
    message.value = t('settings.loadAutoStartFailed', { error: String(err) })
    console.error('Failed to load autostart status:', err)
  }
  finally {
    autoStartLoading.value = false
  }
}

onMounted(async () => {
  await loadSettings()
  await loadPlatformCapabilities()
  await loadAutoStart()
  settingsLoaded.value = true
})

watch([theme, language, notificationDuration, postponeOptions, soundEnabled, soundPreset, soundVolume], () => {
  localStorage.setItem('app-settings', JSON.stringify(buildFrontendSettings()))
  applyTheme(theme.value)
  void setLanguage(language.value).catch((err) => {
    console.error('Failed to save language setting:', err)
  })
  void saveFrontendSettings().catch((err) => {
    console.error('Failed to save frontend settings:', err)
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

watch([dndEnabled, dndStart, dndEnd], async () => {
  try {
    await invoke('save_setting', { key: 'dnd_enabled', value: String(dndEnabled.value) })
    await invoke('save_setting', { key: 'dnd_start', value: dndStart.value })
    await invoke('save_setting', { key: 'dnd_end', value: dndEnd.value })
  }
  catch (err) {
    console.error('Failed to save DND settings:', err)
  }
}, { deep: true })

watch(fullscreenDetectionEnabled, async () => {
  if (!settingsLoaded.value) {
    return
  }

  if (!supportsFullscreenDetection.value) {
    if (fullscreenDetectionEnabled.value) {
      fullscreenDetectionEnabled.value = false
    }
    return
  }

  try {
    await invoke('save_setting', {
      key: 'fullscreen_detection_enabled',
      value: String(fullscreenDetectionEnabled.value),
    })
  }
  catch (err) {
    console.error('Failed to save fullscreen detection setting:', err)
  }
})

function applyTheme(t: string) {
  const root = document.documentElement
  if (t === 'dark') {
    root.setAttribute('data-theme', 'dark')
  }
  else if (t === 'light') {
    root.removeAttribute('data-theme')
  }
  else {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
    if (prefersDark) {
      root.setAttribute('data-theme', 'dark')
    }
    else {
      root.removeAttribute('data-theme')
    }
  }
}

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
    message.value = t('settings.autoStartFailed', { error: String(err) })
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
    message.value = t('settings.silentStartFailed', { error: String(err) })
    console.error('Failed to update silent start:', err)
  }
  finally {
    silentStartLoading.value = false
  }
}

async function handleExport() {
  exporting.value = true
  message.value = ''
  try {
    const data = await exportData()
    data.frontend_settings = buildFrontendSettings()
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
    message.value = t('settings.exportFailed', { error: String(err) })
    console.error('Export failed:', err)
  }
  finally {
    exporting.value = false
  }
}

async function handleImport() {
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

  importing.value = true
  message.value = ''
  try {
    const filePath = await open({
      multiple: false,
      filters: [
        { name: 'JSON', extensions: ['json'] },
      ],
    })

    if (filePath) {
      const content = await readTextFile(filePath)
      const data = JSON.parse(content) as ExportData
      const result = await importData(data, importMode.value)
      if (data.frontend_settings) {
        localStorage.setItem('app-settings', JSON.stringify(data.frontend_settings))
        theme.value = data.frontend_settings.theme
        language.value = data.frontend_settings.language || language.value
        notificationDuration.value = data.frontend_settings.notificationDuration
        postponeOptions.value = data.frontend_settings.postponeOptions
        soundEnabled.value = data.frontend_settings.soundEnabled ?? true
        soundPreset.value = normalizeSoundPreset(data.frontend_settings.soundPreset || 'soft')
        soundVolume.value = normalizeSoundVolume(data.frontend_settings.soundVolume || 60)
        await setLanguage(language.value)
        await saveFrontendSettings()
      }
      await loadSettings()
      message.value = result.message
    }
  }
  catch (err) {
    message.value = t('settings.importFailed', { error: String(err) })
    console.error('Import failed:', err)
  }
  finally {
    importing.value = false
  }
}
</script>

<template>
  <div class="settings-overlay" @click.self="emit('close')">
    <div class="settings-container">
      <div class="settings-header">
        <div class="header-copy">
          <div class="brand-row">
            <img :src="appIconMain" alt="Time Remind" class="brand-icon">
            <div>
              <h2 class="settings-title">
                {{ t('settings.title') }}
              </h2>
              <p class="settings-subtitle">
                {{ t('settings.subtitle') }}
              </p>
            </div>
          </div>
        </div>

        <button class="close-button" type="button" @click="emit('close')">
          <span class="close-symbol">×</span>
        </button>
      </div>

      <div class="settings-content">
        <section class="setting-section">
          <div class="section-heading">
            <h3 class="section-title">
              {{ t('settings.appearance') }}
            </h3>
          </div>

          <div class="theme-selector">
            <button class="theme-button" :class="{ 'theme-button-active': theme === 'light' }" type="button" @click="theme = 'light'">
              <span>{{ t('settings.light') }}</span>
            </button>
            <button class="theme-button" :class="{ 'theme-button-active': theme === 'dark' }" type="button" @click="theme = 'dark'">
              <span>{{ t('settings.dark') }}</span>
            </button>
            <button class="theme-button" :class="{ 'theme-button-active': theme === 'system' }" type="button" @click="theme = 'system'">
              <span>{{ t('settings.systemTheme') }}</span>
            </button>
          </div>

          <div class="setting-row language-row">
            <label class="setting-label">{{ t('settings.language') }}</label>
            <select v-model="language" class="setting-input language-select">
              <option v-for="option in languageOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </div>
        </section>

        <section class="setting-section">
          <div class="section-heading">
            <h3 class="section-title">
              {{ t('settings.system') }}
            </h3>
          </div>

          <div class="setting-row setting-card platform-capability-card">
            <div class="setting-card-main">
              <div class="setting-card-copy">
                <label class="setting-label">{{ t('settings.platformCapabilities') }}</label>
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
                    v-model="autoStart"
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
                      v-model="silentStart"
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
              <input v-model="fullscreenDetectionEnabled" :disabled="fullscreenDetectionDisabled" type="checkbox">
              <span class="slider" />
            </label>
          </div>
        </section>

        <section class="setting-section">
          <div class="section-heading">
            <h3 class="section-title">
              {{ t('settings.notification') }}
            </h3>
          </div>

          <div class="setting-row">
            <label class="setting-label">{{ t('settings.duration') }}</label>
            <input v-model.number="notificationDuration" class="setting-input" type="number" min="5" max="120">
          </div>

          <div class="setting-row sound-row">
            <label class="setting-label">{{ t('settings.sound') }}</label>
            <label class="switch">
              <input v-model="soundEnabled" type="checkbox">
              <span class="slider" />
            </label>
          </div>

          <Transition name="slide-down">
            <div v-if="soundEnabled" class="sound-panel">
              <div class="setting-row">
                <label class="setting-label">{{ t('settings.soundPreset') }}</label>
                <select v-model="soundPreset" class="setting-input">
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
              </div>

              <div class="setting-row">
                <label class="setting-label">{{ t('settings.soundVolume') }}</label>
                <input v-model.number="soundVolume" class="sound-range" type="range" min="0" max="100">
                <span class="sound-volume">{{ soundVolume }}%</span>
              </div>

              <button class="data-button sound-test-button" type="button" @click="testNotificationSound">
                {{ t('settings.testSound') }}
              </button>
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
              <input v-model="dndEnabled" type="checkbox">
              <span class="slider" />
            </label>
          </div>

          <Transition name="slide-down">
            <div v-if="dndEnabled" class="time-range">
              <div class="time-field">
                <label class="time-label">{{ t('settings.startTime') }}</label>
                <input v-model="dndStart" class="setting-input" type="time">
              </div>
              <div class="time-field">
                <label class="time-label">{{ t('settings.endTime') }}</label>
                <input v-model="dndEnd" class="setting-input" type="time">
              </div>
            </div>
          </Transition>
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
          <p v-if="message" class="message" :class="{ 'message-error': message.toLowerCase().includes(t('settings.failedKeyword')) }">
            {{ message }}
          </p>
        </section>
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
  padding: 20px;
  background: rgba(7, 10, 16, 0.48);
  backdrop-filter: blur(8px);
}

.settings-container {
  width: min(100%, 640px);
  max-height: 86vh;
  overflow: hidden;
  border-radius: 28px;
  border: 1px solid rgba(148, 163, 184, 0.16);
  background:
    radial-gradient(circle at top right, rgba(47, 159, 216, 0.12), transparent 28%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(255, 255, 255, 0.92));
  box-shadow: 0 30px 60px rgba(15, 23, 42, 0.22);
  display: flex;
  flex-direction: column;
}

[data-theme='dark'] .settings-container {
  background:
    radial-gradient(circle at top right, rgba(47, 159, 216, 0.16), transparent 28%),
    linear-gradient(180deg, rgba(24, 28, 37, 0.96), rgba(16, 20, 28, 0.94));
}

.settings-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 24px 24px 18px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.14);
}

.brand-row {
  display: flex;
  align-items: center;
  gap: 14px;
}

.brand-icon {
  width: 42px;
  height: 42px;
  flex-shrink: 0;
}

.settings-title {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
}

.settings-subtitle {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.close-button {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  background: rgba(148, 163, 184, 0.12);
  transition: transform 0.2s ease, background-color 0.2s ease;
}

.close-button:hover {
  background: rgba(148, 163, 184, 0.18);
}

.close-button:active {
  transform: scale(0.94);
}

.close-symbol {
  font-size: 20px;
  line-height: 1;
}

.settings-content {
  padding: 0 24px 24px;
  overflow-y: auto;
}

.setting-section {
  padding-top: 22px;
}

.section-heading {
  display: flex;
  align-items: center;
  margin-bottom: 14px;
}

.section-title {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-secondary);
}

.theme-selector {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.language-row {
  margin-top: 12px;
}

.language-select {
  min-width: 160px;
}

.theme-button,
.mode-button,
.data-button {
  border-radius: 18px;
  border: 1px solid rgba(148, 163, 184, 0.16);
  background: rgba(255, 255, 255, 0.72);
  color: var(--text-primary);
  transition:
    transform 0.18s ease,
    border-color 0.18s ease,
    background-color 0.18s ease,
    box-shadow 0.18s ease;
}

[data-theme='dark'] .theme-button,
[data-theme='dark'] .mode-button,
[data-theme='dark'] .data-button {
  background: rgba(24, 29, 38, 0.84);
}

.theme-button {
  padding: 14px 10px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
}

.theme-button-active,
.mode-button-active {
  border-color: rgba(47, 159, 216, 0.26);
  background: rgba(47, 159, 216, 0.12);
  box-shadow: 0 12px 24px rgba(15, 23, 42, 0.08);
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  border-radius: 20px;
  border: 1px solid rgba(148, 163, 184, 0.14);
  background: rgba(255, 255, 255, 0.66);
}

[data-theme='dark'] .setting-row {
  background: rgba(24, 29, 38, 0.84);
}

.setting-label,
.time-label {
  font-size: 14px;
  color: var(--text-primary);
}

.setting-card {
  align-items: stretch;
  margin-bottom: 12px;
}

.setting-card-main {
  width: 100%;
}

.setting-card-header,
.setting-subcard {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.setting-card-copy,
.setting-subcard-copy,
.setting-inline-copy {
  display: flex;
  flex-direction: column;
}

.setting-subcard {
  margin-top: 12px;
  padding: 14px 16px;
  border-radius: 16px;
  border: 1px solid rgba(47, 159, 216, 0.14);
  background: rgba(47, 159, 216, 0.08);
}

[data-theme='dark'] .setting-subcard {
  background: rgba(47, 159, 216, 0.1);
  border-color: rgba(47, 159, 216, 0.2);
}

.setting-description {
  margin: 4px 0 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.setting-warning {
  color: #b7791f;
}

[data-theme='dark'] .setting-warning {
  color: #f6c76f;
}

.setting-input {
  padding: 10px 12px;
  border-radius: 14px;
  border: 1px solid rgba(148, 163, 184, 0.22);
  background: rgba(255, 255, 255, 0.78);
  color: var(--text-primary);
  outline: none;
}

[data-theme='dark'] .setting-input {
  background: rgba(20, 24, 31, 0.88);
}

.setting-input:focus {
  border-color: rgba(47, 159, 216, 0.4);
  box-shadow: 0 0 0 4px rgba(47, 159, 216, 0.12);
}

.switch {
  position: relative;
  display: inline-block;
  width: 48px;
  height: 26px;
}

.switch-disabled {
  cursor: not-allowed;
  opacity: 0.62;
}

.switch-loading {
  cursor: wait;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  inset: 0;
  border-radius: 999px;
  background-color: rgba(148, 163, 184, 0.36);
  transition: background-color 0.2s ease;
}

.slider::before {
  content: '';
  position: absolute;
  left: 3px;
  bottom: 3px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: white;
  box-shadow: 0 2px 6px rgba(15, 23, 42, 0.22);
  transition: transform 0.22s ease;
}

input:checked + .slider {
  background-color: var(--color-primary);
}

input:checked + .slider::before {
  transform: translateX(22px);
}

.time-range {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  margin-top: 12px;
}

.sound-panel {
  display: grid;
  gap: 12px;
  margin-top: 12px;
}

.sound-row {
  margin-top: 12px;
}

.sound-range {
  flex: 1;
  min-width: 140px;
  accent-color: var(--color-primary);
}

.sound-volume {
  min-width: 48px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  text-align: right;
}

.sound-test-button {
  width: 100%;
}

.time-field {
  padding: 14px 16px;
  border-radius: 20px;
  border: 1px solid rgba(148, 163, 184, 0.14);
  background: rgba(255, 255, 255, 0.66);
}

[data-theme='dark'] .time-field {
  background: rgba(24, 29, 38, 0.84);
}

.time-label {
  display: block;
  margin-bottom: 10px;
}

.import-mode,
.data-actions {
  display: flex;
  gap: 10px;
}

.mode-button {
  flex: 1;
  padding: 12px 14px;
  font-size: 13px;
  font-weight: 600;
}

.data-actions {
  margin-top: 12px;
}

.data-button {
  flex: 1;
  padding: 14px 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
}

.theme-button:hover,
.mode-button:hover:not(:disabled),
.data-button:hover:not(:disabled) {
  transform: translateY(-1px);
  border-color: rgba(47, 159, 216, 0.26);
  box-shadow: 0 12px 24px rgba(15, 23, 42, 0.08);
}

.theme-button:active,
.mode-button:active:not(:disabled),
.data-button:active:not(:disabled) {
  transform: scale(0.97);
}

.mode-button:disabled,
.data-button:disabled {
  opacity: 0.58;
  cursor: not-allowed;
}

.hint {
  margin: 10px 0 0;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.message {
  margin: 12px 0 0;
  padding: 12px 14px;
  border-radius: 16px;
  background: rgba(47, 159, 216, 0.12);
  color: var(--text-primary);
  font-size: 13px;
  line-height: 1.6;
}

.message-error {
  background: rgba(227, 93, 106, 0.12);
  color: #d95767;
}

.slide-down-enter-active,
.slide-down-leave-active {
  transition:
    opacity 0.22s ease,
    transform 0.22s ease;
}

.slide-down-enter-from,
.slide-down-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

@media (max-width: 640px) {
  .theme-selector,
  .time-range,
  .import-mode,
  .data-actions {
    grid-template-columns: 1fr;
    display: grid;
  }

  .setting-row {
    flex-direction: column;
    align-items: flex-start;
  }

  .setting-card-header,
  .setting-subcard {
    width: 100%;
  }
}
</style>
