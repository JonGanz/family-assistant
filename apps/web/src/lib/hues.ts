import { HUES, type Hue } from '../themes/types'
import { textColorFor } from './contrast'

export { HUES }
export type { Hue }

export const isHue = (x: unknown): x is Hue =>
  typeof x === 'string' && (HUES as readonly string[]).includes(x)

/** The CSS colour of a member hue; an unset (or unknown) hue is blue. */
export const hueVar = (hue: Hue | string | null | undefined): string =>
  `var(--hue-${isHue(hue) ? hue : 'blue'})`

/** Text colour on a fill: the theme's background for hue variables, computed for calendar hex colours. */
export const textOn = (fill: string): string =>
  /^#[0-9a-fA-F]{6}$/.test(fill) ? textColorFor(fill) : 'var(--color-bg)'
