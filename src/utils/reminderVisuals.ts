import type { Language } from '../i18n/messages'
import appIconMain from '../assets/icons/app/app-icon-main.png'
import reminderDrink from '../assets/icons/reminder/reminder-drink.png'
import reminderEyeCare from '../assets/icons/reminder/reminder-eye-care.png'
import reminderRest from '../assets/icons/reminder/reminder-rest.png'
import emptyReminders from '../assets/illustrations/empty/empty-reminders.png'
import catDrink from '../assets/illustrations/mascot/cat-drink.png'
import catEyeCare from '../assets/illustrations/mascot/cat-eye-care.png'
import catRest from '../assets/illustrations/mascot/cat-rest.png'
import catSnooze from '../assets/illustrations/mascot/cat-snooze.png'
import { messages } from '../i18n/messages'

export type ReminderTypeKey = 'drink' | 'rest' | 'eye_care' | 'custom'

export interface ReminderVisual {
  type: ReminderTypeKey
  label: string
  description: string
  shortMessage: string
  defaultName: string
  defaultIntervalMinutes: number
  defaultBreakDurationMinutes?: number
  defaultBreakNotificationEnabled?: boolean
  defaultActionDurationSeconds?: number
  accent: string
  accentSoft: string
  borderSoft: string
  badgeBackground: string
  iconAsset?: string
  mascotAsset?: string
  statusAsset?: string
  iconText?: string
}

const reminderVisualMap: Record<ReminderTypeKey, ReminderVisual> = {
  drink: {
    type: 'drink',
    label: 'Drink reminder',
    description: 'Keep a steady hydration rhythm.',
    shortMessage: 'Have some water and relax.',
    defaultName: 'Drink reminder',
    defaultIntervalMinutes: 90,
    accent: '#2F9FD8',
    accentSoft: 'rgba(47, 159, 216, 0.18)',
    borderSoft: 'rgba(47, 159, 216, 0.26)',
    badgeBackground: 'linear-gradient(180deg, rgba(47, 159, 216, 0.18), rgba(47, 159, 216, 0.08))',
    iconAsset: reminderDrink,
    mascotAsset: catDrink,
    statusAsset: catSnooze,
  },
  rest: {
    type: 'rest',
    label: 'Rest reminder',
    description: 'Stand up and stretch for a while.',
    shortMessage: 'Stand up and move for two minutes.',
    defaultName: 'Rest reminder',
    defaultIntervalMinutes: 60,
    defaultBreakDurationMinutes: 5,
    defaultBreakNotificationEnabled: true,
    defaultActionDurationSeconds: 300,
    accent: '#57C59A',
    accentSoft: 'rgba(87, 197, 154, 0.18)',
    borderSoft: 'rgba(87, 197, 154, 0.26)',
    badgeBackground: 'linear-gradient(180deg, rgba(87, 197, 154, 0.18), rgba(87, 197, 154, 0.08))',
    iconAsset: reminderRest,
    mascotAsset: catRest,
    statusAsset: catSnooze,
  },
  eye_care: {
    type: 'eye_care',
    label: 'Eye care reminder',
    description: 'Look away from the screen for a moment.',
    shortMessage: 'Look 20 feet away for 20 seconds.',
    defaultName: 'Eye care reminder',
    defaultIntervalMinutes: 20,
    defaultActionDurationSeconds: 20,
    accent: '#F6B35B',
    accentSoft: 'rgba(246, 179, 91, 0.18)',
    borderSoft: 'rgba(246, 179, 91, 0.26)',
    badgeBackground: 'linear-gradient(180deg, rgba(246, 179, 91, 0.18), rgba(246, 179, 91, 0.08))',
    iconAsset: reminderEyeCare,
    mascotAsset: catEyeCare,
    statusAsset: catSnooze,
  },
  custom: {
    type: 'custom',
    label: 'Custom reminder',
    description: 'Track your own rhythm and actions.',
    shortMessage: 'Leave yourself a gentle reminder.',
    defaultName: 'Custom reminder',
    defaultIntervalMinutes: 60,
    accent: '#7A8CA8',
    accentSoft: 'rgba(122, 140, 168, 0.18)',
    borderSoft: 'rgba(122, 140, 168, 0.26)',
    badgeBackground: 'linear-gradient(180deg, rgba(122, 140, 168, 0.18), rgba(122, 140, 168, 0.08))',
    iconText: 'C',
  },
}

export const reminderTypeOptions = [
  reminderVisualMap.drink,
  reminderVisualMap.rest,
  reminderVisualMap.eye_care,
  reminderVisualMap.custom,
]

export function getReminderVisual(type: string): ReminderVisual {
  return reminderVisualMap[type as ReminderTypeKey] ?? reminderVisualMap.custom
}

export function getLocalizedReminderVisual(type: string, language: Language): ReminderVisual {
  const visual = getReminderVisual(type)
  const localizedMap = messages[language].reminderTypes
  const localized = visual.type === 'eye_care'
    ? localizedMap.eyeCare
    : localizedMap[visual.type]

  return {
    ...visual,
    label: localized.label,
    description: localized.description,
    shortMessage: localized.shortMessage,
    defaultName: localized.defaultName,
    iconText: visual.type === 'custom' ? localizedMap.custom.iconText : visual.iconText,
  }
}

export function getLocalizedReminderTypeOptions(language: Language): ReminderVisual[] {
  return reminderTypeOptions.map(option => getLocalizedReminderVisual(option.type, language))
}

export { appIconMain, emptyReminders }
