export const HUES = ['red', 'orange', 'yellow', 'green', 'cyan', 'blue', 'purple', 'pink'] as const
export type Hue = (typeof HUES)[number]
export type Mode = 'light' | 'dark'

export interface ThemeVariant {
  id: string
  label: string
  mode: Mode
  neutrals: {
    bg: string
    surface: string
    surface2: string
    border: string
    muted: string
    text: string
    onAccent: string
  }
  hues: Record<Hue, string>
  /** Richer palette from the Neovim ports (R-006, D-106); `layers[0]` equals `neutrals.bg`. */
  extended?: {
    layers: [string, string, string, string]
    textStrong?: string
    textSubtle?: string
    comment?: string
    selection?: string
    highlight?: string
    accents?: Record<string, string>
  }
  /** Percent of `text` mixed into `bg` for layers 1..3 when `extended` is absent (default 5/10/15). */
  layerMix?: [number, number, number]
}

export interface ThemeFamily {
  id: string
  label: string
  source: string
  license: string
  /** The first variant of each mode is that mode's default. */
  variants: ThemeVariant[]
}

export interface ThemeChoice {
  family: string
  variant?: string
  mode: 'auto' | 'light' | 'dark'
}
