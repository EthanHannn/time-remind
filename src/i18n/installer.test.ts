import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { languageOptions } from './messages'

const read = (path: string) => readFileSync(new URL(`../..${path}`, import.meta.url), 'utf8')
const installerLanguages: Record<string, string> = {
  'en-US': 'English',
  'zh-CN': 'SimpChinese',
  'zh-TW': 'TradChinese',
  'ja-JP': 'Japanese',
  'ko-KR': 'Korean',
  'fr-FR': 'French',
  'de-DE': 'German',
  'vi-VN': 'Vietnamese',
  'th-TH': 'Thai',
  'ms-MY': 'Malay',
  'km-KH': 'Khmer',
  'es-ES': 'Spanish',
  'pt-BR': 'PortugueseBR',
  'id-ID': 'Indonesian',
  'it-IT': 'Italian',
  'ru-RU': 'Russian',
  'tr-TR': 'Turkish',
  'ar': 'Arabic',
  'hi-IN': 'Hindi',
}

describe('installer language coverage', () => {
  it('offers every app language, startup choices and the corresponding first-run marker', () => {
    const config = JSON.parse(read('/src-tauri/tauri.conf.json')).bundle.windows.nsis
    expect(config.languages.sort()).toEqual(Object.values(installerLanguages).sort())
    expect(languageOptions.map(option => option.value).sort()).toEqual(Object.keys(installerLanguages).sort())
    const choices = read('/src-tauri/nsis/startup-languages.nsh')
    const hook = read('/src-tauri/nsis/hooks.nsh')
    for (const [locale, installer] of Object.entries(installerLanguages)) {
      expect(choices).toContain(`!insertmacro StartupLanguage \${LANG_${installer.toUpperCase()}}`)
      expect(hook).toContain(`StrCpy $R0 "${locale}"`)
    }
  })

  it('provides all Tauri messages and interpolation parameters in custom languages', () => {
    const strings = (content: string) => Object.fromEntries([...content.matchAll(/^LangString (\w+) \S+ "(.*)"/gm)].map(match => [match[1], match[2]!]))
    const reference = strings(read('/src-tauri/nsis/languages/Korean.nsh'))
    const params = (value: string) => [...value.matchAll(/\$\{\w+\}|\$R\d|\$\d|\{\{\w+\}\}/g)].map(match => match[0]).sort()
    for (const name of ['Khmer', 'Indonesian', 'Hindi']) {
      const actual = strings(read(`/src-tauri/nsis/languages/${name}.nsh`))
      expect(Object.keys(actual).sort()).toEqual(Object.keys(reference).sort())
      for (const [key, value] of Object.entries(actual)) {
        expect(value.trim()).not.toBe('')
        expect(params(value), `${name}.${key}`).toEqual(params(reference[key]!))
      }
    }
    expect(read('/src-tauri/nsis/languages/native/Khmer.nlf')).toMatch(/Language ID\r?\n1107/)
    expect(read('/src-tauri/nsis/languages/native/Khmer.nsh')).toContain('MUI_TEXT_FINISH_INFO_TITLE')
    // The interpolation below belongs to NSIS, not JavaScript.
    // eslint-disable-next-line no-template-curly-in-string
    expect(read('/src-tauri/nsis/installer.nsi')).toContain('!insertmacro MUI_LANGUAGEEX "${TIME_REMIND_NSIS_DIR}\\languages\\native" "Khmer"')
  })
})
