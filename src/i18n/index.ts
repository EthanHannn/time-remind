import type { Language, MessageSchema } from './messages'
import { invoke } from '@tauri-apps/api/core'
import { computed, shallowRef } from 'vue'
import { languageOptions, messages } from './messages'

const STORAGE_KEY = 'app-language'
const language = shallowRef<Language>(resolveSystemLanguage())

function matchLanguage(value: string): Language | null {
  const normalized = value.toLowerCase().replace('_', '-')
  const exact = languageOptions.find(option => option.value.toLowerCase() === normalized)
  if (exact)
    return exact.value

  if (normalized.startsWith('zh-hant') || normalized === 'zh-tw' || normalized === 'zh-hk' || normalized === 'zh-mo')
    return 'zh-TW'
  if (normalized.startsWith('zh'))
    return 'zh-CN'
  if (normalized.startsWith('en'))
    return 'en-US'
  if (normalized.startsWith('ja'))
    return 'ja-JP'
  if (normalized.startsWith('ko'))
    return 'ko-KR'
  if (normalized.startsWith('fr'))
    return 'fr-FR'
  if (normalized.startsWith('de'))
    return 'de-DE'
  if (normalized.startsWith('vi'))
    return 'vi-VN'
  if (normalized.startsWith('th'))
    return 'th-TH'
  if (normalized.startsWith('ms'))
    return 'ms-MY'
  if (normalized.startsWith('km'))
    return 'km-KH'

  return null
}

function resolveSystemLanguage(): Language {
  if (typeof navigator === 'undefined')
    return 'en-US'

  const candidates = navigator.languages?.length ? navigator.languages : [navigator.language]
  for (const candidate of candidates) {
    const matched = matchLanguage(candidate)
    if (matched)
      return matched
  }

  return 'en-US'
}

function resolveNestedMessage(source: unknown, path: string): string | null {
  const segments = path.split('.')
  let current = source

  for (const segment of segments) {
    if (!current || typeof current !== 'object' || !(segment in current)) {
      return null
    }

    current = (current as Record<string, unknown>)[segment]
  }

  return typeof current === 'string' ? current : null
}

function getNestedMessage(path: string): string {
  return resolveNestedMessage(messages[language.value], path)
    ?? resolveNestedMessage(messages['en-US'], path)
    ?? path
}

export function useI18n() {
  const locale = computed(() => language.value)
  const m = computed<MessageSchema>(() => messages[language.value])

  function t(path: string, params: Record<string, string | number> = {}) {
    return Object.entries(params).reduce(
      (text, [key, value]) => text.split(`{${key}}`).join(String(value)),
      getNestedMessage(path),
    )
  }

  async function setLanguage(nextLanguage: Language) {
    language.value = nextLanguage
    localStorage.setItem(STORAGE_KEY, nextLanguage)
    await invoke('save_setting', { key: 'language', value: nextLanguage })
  }

  return {
    language,
    locale,
    languageOptions,
    m,
    t,
    setLanguage,
  }
}

export async function loadLanguage(settings?: Record<string, string>) {
  const settingLanguage = settings?.language ? matchLanguage(settings.language) : null
  if (settingLanguage) {
    language.value = settingLanguage
    localStorage.setItem(STORAGE_KEY, settingLanguage)
    return
  }

  try {
    const dbSettings = settings ?? await invoke<Record<string, string>>('get_all_settings')
    const dbLanguage = dbSettings.language ? matchLanguage(dbSettings.language) : null
    if (dbLanguage) {
      language.value = dbLanguage
      localStorage.setItem(STORAGE_KEY, dbLanguage)
      return
    }
  }
  catch {
    // 忽略设置读取失败，回退到本地缓存
  }

  const saved = localStorage.getItem(STORAGE_KEY)
  const savedLanguage = saved ? matchLanguage(saved) : null
  if (savedLanguage) {
    language.value = savedLanguage
    return
  }

  const legacy = localStorage.getItem('app-settings')
  if (!legacy) {
    language.value = resolveSystemLanguage()
    return
  }

  try {
    const parsed = JSON.parse(legacy) as { language?: unknown }
    if (typeof parsed.language === 'string') {
      const legacyLanguage = matchLanguage(parsed.language)
      if (legacyLanguage) {
        language.value = legacyLanguage
        return
      }
    }
  }
  catch {
    // 忽略旧设置解析失败
  }

  language.value = resolveSystemLanguage()
}

export type { Language }
