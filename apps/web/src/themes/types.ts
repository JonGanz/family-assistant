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
