import catSiameseDrink from '../assets/illustrations/mascot/cat-siamese/drink.webp'
import catSiameseEyeCare from '../assets/illustrations/mascot/cat-siamese/eye-care.webp'
import catSiameseRest from '../assets/illustrations/mascot/cat-siamese/rest.webp'
import catSiameseSnooze from '../assets/illustrations/mascot/cat-siamese/snooze.webp'
import catSilverDrink from '../assets/illustrations/mascot/cat-silver/drink.webp'
import catSilverEyeCare from '../assets/illustrations/mascot/cat-silver/eye-care.webp'
import catSilverRest from '../assets/illustrations/mascot/cat-silver/rest.webp'
import catSilverSnooze from '../assets/illustrations/mascot/cat-silver/snooze.webp'
import classicDrink from '../assets/illustrations/mascot/classic/drink.webp'
import classicEyeCare from '../assets/illustrations/mascot/classic/eye-care.webp'
import classicRest from '../assets/illustrations/mascot/classic/rest.webp'
import classicSnooze from '../assets/illustrations/mascot/classic/snooze.webp'
import editorialDrink from '../assets/illustrations/mascot/editorial/drink.webp'
import editorialEyeCare from '../assets/illustrations/mascot/editorial/eye-care.webp'
import editorialRest from '../assets/illustrations/mascot/editorial/rest.webp'
import editorialSnooze from '../assets/illustrations/mascot/editorial/snooze.webp'
import mechaNuDrink from '../assets/illustrations/mascot/mecha-nu/drink.webp'
import mechaNuEyeCare from '../assets/illustrations/mascot/mecha-nu/eye-care.webp'
import mechaNuRest from '../assets/illustrations/mascot/mecha-nu/rest.webp'
import mechaNuSnooze from '../assets/illustrations/mascot/mecha-nu/snooze.webp'
import mechaSazabiDrink from '../assets/illustrations/mascot/mecha-sazabi/drink.webp'
import mechaSazabiEyeCare from '../assets/illustrations/mascot/mecha-sazabi/eye-care.webp'
import mechaSazabiRest from '../assets/illustrations/mascot/mecha-sazabi/rest.webp'
import mechaSazabiSnooze from '../assets/illustrations/mascot/mecha-sazabi/snooze.webp'
import shibaBlackDrink from '../assets/illustrations/mascot/shiba-black/drink.webp'
import shibaBlackEyeCare from '../assets/illustrations/mascot/shiba-black/eye-care.webp'
import shibaBlackRest from '../assets/illustrations/mascot/shiba-black/rest.webp'
import shibaBlackSnooze from '../assets/illustrations/mascot/shiba-black/snooze.webp'
import shibaCreamDrink from '../assets/illustrations/mascot/shiba-cream/drink.webp'
import shibaCreamEyeCare from '../assets/illustrations/mascot/shiba-cream/eye-care.webp'
import shibaCreamRest from '../assets/illustrations/mascot/shiba-cream/rest.webp'
import shibaCreamSnooze from '../assets/illustrations/mascot/shiba-cream/snooze.webp'
import watercolorDrink from '../assets/illustrations/mascot/watercolor/drink.webp'
import watercolorEyeCare from '../assets/illustrations/mascot/watercolor/eye-care.webp'
import watercolorRest from '../assets/illustrations/mascot/watercolor/rest.webp'
import watercolorSnooze from '../assets/illustrations/mascot/watercolor/snooze.webp'

// Legacy persisted IDs ('classic' | 'editorial' | 'watercolor') are retained for
// existing settings and imported backups:
// - classic     -> Calico cat (fluffy oil-pastel)
// - editorial   -> Red Shiba Inu (marshmallow gouache)
// - watercolor  -> MSZ-010 ZZ Gundam (chunky hand-drawn anime)
export type MascotStyle
  = | 'classic'
    | 'cat_silver'
    | 'cat_siamese'
    | 'editorial'
    | 'shiba_black'
    | 'shiba_cream'
    | 'watercolor'
    | 'mecha_sazabi'
    | 'mecha_nu'

export type MascotGroup = 'cat' | 'shiba' | 'mecha'
export type MascotState = 'drink' | 'rest' | 'eye_care' | 'snooze'
export type PreviewReminderType = Exclude<MascotState, 'snooze'> | 'custom'

export interface MascotOption {
  readonly id: MascotStyle
  readonly group: MascotGroup
  readonly image: string
  readonly nameKey: string
  readonly previewState: Exclude<MascotState, 'snooze'>
}

export const mascotGroupOptions: readonly { readonly id: MascotGroup, readonly labelKey: string }[] = [
  { id: 'cat', labelKey: 'ui.groupCat' },
  { id: 'shiba', labelKey: 'ui.groupShiba' },
  { id: 'mecha', labelKey: 'ui.groupMecha' },
]

export const mascotStyleOptions: readonly MascotOption[] = [
  { id: 'classic', group: 'cat', image: classicDrink, nameKey: 'ui.mascotCatCalico', previewState: 'drink' },
  { id: 'cat_silver', group: 'cat', image: catSilverDrink, nameKey: 'ui.mascotCatSilver', previewState: 'drink' },
  { id: 'cat_siamese', group: 'cat', image: catSiameseDrink, nameKey: 'ui.mascotCatSiamese', previewState: 'drink' },
  { id: 'editorial', group: 'shiba', image: editorialRest, nameKey: 'ui.mascotShibaRed', previewState: 'rest' },
  { id: 'shiba_black', group: 'shiba', image: shibaBlackRest, nameKey: 'ui.mascotShibaBlack', previewState: 'rest' },
  { id: 'shiba_cream', group: 'shiba', image: shibaCreamRest, nameKey: 'ui.mascotShibaCream', previewState: 'rest' },
  { id: 'watercolor', group: 'mecha', image: watercolorEyeCare, nameKey: 'ui.mascotMechaZz', previewState: 'eye_care' },
  { id: 'mecha_sazabi', group: 'mecha', image: mechaSazabiEyeCare, nameKey: 'ui.mascotMechaSazabi', previewState: 'eye_care' },
  { id: 'mecha_nu', group: 'mecha', image: mechaNuEyeCare, nameKey: 'ui.mascotMechaNu', previewState: 'eye_care' },
]

const assets: Record<MascotStyle, Record<MascotState, string>> = {
  classic: { drink: classicDrink, rest: classicRest, eye_care: classicEyeCare, snooze: classicSnooze },
  cat_silver: { drink: catSilverDrink, rest: catSilverRest, eye_care: catSilverEyeCare, snooze: catSilverSnooze },
  cat_siamese: { drink: catSiameseDrink, rest: catSiameseRest, eye_care: catSiameseEyeCare, snooze: catSiameseSnooze },
  editorial: { drink: editorialDrink, rest: editorialRest, eye_care: editorialEyeCare, snooze: editorialSnooze },
  shiba_black: { drink: shibaBlackDrink, rest: shibaBlackRest, eye_care: shibaBlackEyeCare, snooze: shibaBlackSnooze },
  shiba_cream: { drink: shibaCreamDrink, rest: shibaCreamRest, eye_care: shibaCreamEyeCare, snooze: shibaCreamSnooze },
  watercolor: { drink: watercolorDrink, rest: watercolorRest, eye_care: watercolorEyeCare, snooze: watercolorSnooze },
  mecha_sazabi: { drink: mechaSazabiDrink, rest: mechaSazabiRest, eye_care: mechaSazabiEyeCare, snooze: mechaSazabiSnooze },
  mecha_nu: { drink: mechaNuDrink, rest: mechaNuRest, eye_care: mechaNuEyeCare, snooze: mechaNuSnooze },
}

const validStyles = new Set<string>(Object.keys(assets))
const groupByStyle = Object.fromEntries(mascotStyleOptions.map(option => [option.id, option.group])) as Record<MascotStyle, MascotGroup>

export function normalizeMascotStyle(value: unknown): MascotStyle {
  return typeof value === 'string' && validStyles.has(value) ? value as MascotStyle : 'classic'
}

export function getMascotGroup(style: MascotStyle): MascotGroup {
  return groupByStyle[normalizeMascotStyle(style)]
}

export function getMascotAsset(style: MascotStyle, state: MascotState): string {
  return assets[normalizeMascotStyle(style)][state]
}
