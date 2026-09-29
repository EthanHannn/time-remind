import classicDrink from '../assets/illustrations/mascot/cat-drink.png'
import classicEyeCare from '../assets/illustrations/mascot/cat-eye-care.png'
import classicRest from '../assets/illustrations/mascot/cat-rest.png'
import classicSnooze from '../assets/illustrations/mascot/cat-snooze.png'
import editorialDrink from '../assets/illustrations/mascot/editorial/drink.webp'
import editorialEyeCare from '../assets/illustrations/mascot/editorial/eye-care.webp'
import editorialRest from '../assets/illustrations/mascot/editorial/rest.webp'
import editorialSnooze from '../assets/illustrations/mascot/editorial/snooze.webp'
import watercolorDrink from '../assets/illustrations/mascot/watercolor/drink.webp'
import watercolorEyeCare from '../assets/illustrations/mascot/watercolor/eye-care.webp'
import watercolorRest from '../assets/illustrations/mascot/watercolor/rest.webp'
import watercolorSnooze from '../assets/illustrations/mascot/watercolor/snooze.webp'

// Persisted IDs are retained for existing settings and imported backups.
// Their current artwork is flat, line drawing, and paper cut respectively.
export type MascotStyle = 'classic' | 'editorial' | 'watercolor'
export type MascotState = 'drink' | 'rest' | 'eye_care' | 'snooze'
export type PreviewReminderType = Exclude<MascotState, 'snooze'>

export const mascotStyleOptions = [
  { id: 'classic', image: classicDrink, nameKey: 'settings.mascotClassic', descriptionKey: 'settings.mascotClassicDescription' },
  { id: 'editorial', image: editorialDrink, nameKey: 'settings.mascotEditorial', descriptionKey: 'settings.mascotEditorialDescription' },
  { id: 'watercolor', image: watercolorDrink, nameKey: 'settings.mascotWatercolor', descriptionKey: 'settings.mascotWatercolorDescription' },
] as const

const assets: Record<MascotStyle, Record<MascotState, string>> = {
  classic: { drink: classicDrink, rest: classicRest, eye_care: classicEyeCare, snooze: classicSnooze },
  editorial: { drink: editorialDrink, rest: editorialRest, eye_care: editorialEyeCare, snooze: editorialSnooze },
  watercolor: { drink: watercolorDrink, rest: watercolorRest, eye_care: watercolorEyeCare, snooze: watercolorSnooze },
}

export function normalizeMascotStyle(value: unknown): MascotStyle {
  return value === 'editorial' || value === 'watercolor' ? value : 'classic'
}

export function getMascotAsset(style: MascotStyle, state: MascotState): string {
  return assets[normalizeMascotStyle(style)][state]
}
