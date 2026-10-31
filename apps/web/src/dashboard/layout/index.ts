import type { ThemeChoice } from '../../themes'

export type PanelKind =
  'clock' | 'weather' | 'agenda' | 'week' | 'members' | 'now' | 'member_tasks' | 'routine'
/** The shape of the screen the dashboard is on; detected, not configured. */
export type Orientation = 'landscape' | 'portrait'

export interface PanelConfig {
  kind: string
  [key: string]: unknown
}

export interface DashboardSettings {
  theme: ThemeChoice
  agenda_days: number
  /** Minutes without a touch before the screensaver starts; 0 is off. Absent means the default. */
  screensaver_minutes?: number
  members: 'all' | string[]
  panels: PanelConfig[]
}

export interface GridArea {
  panel: PanelConfig
  column: string
  row: string
}

export const KNOWN_PANELS: readonly string[] = [
  'clock',
  'weather',
  'agenda',
  'week',
  'members',
  'now',
  'member_tasks',
  'routine',
]

const base = {
  theme: { family: 'catppuccin', mode: 'auto' },
  members: 'all',
} as const

export const PRESETS: Record<string, DashboardSettings> = {
  'Family hub': {
    ...base,
    agenda_days: 3,
    panels: [
      { kind: 'clock' },
      { kind: 'weather', hourly: true },
      { kind: 'agenda' },
      { kind: 'now' },
      { kind: 'routine' },
    ],
  },
  'Chores board': {
    ...base,
    agenda_days: 3,
    panels: [{ kind: 'clock' }, { kind: 'weather', hourly: false }, { kind: 'member_tasks' }],
  },
  'Calendar wall': {
    ...base,
    agenda_days: 7,
    panels: [{ kind: 'clock' }, { kind: 'weather', hourly: false }, { kind: 'week' }],
  },
  Minimal: { ...base, agenda_days: 2, panels: [{ kind: 'clock' }, { kind: 'agenda' }] },
}

/**
 * Reads a saved theme: the legacy string (`"dark"`) becomes the Catppuccin choice, an object is
 * kept as it is (unknown families too; the renderer falls back), anything else is the default.
 */
export function normalizeTheme(raw: unknown): ThemeChoice {
  if (raw === 'auto' || raw === 'light' || raw === 'dark') {
    return { family: 'catppuccin', mode: raw }
  }
  if (raw && typeof raw === 'object') {
    const r = raw as Partial<ThemeChoice>
    if (typeof r.family === 'string') {
      const mode = r.mode === 'light' || r.mode === 'dark' ? r.mode : 'auto'
      return {
        family: r.family,
        mode,
        ...(typeof r.variant === 'string' ? { variant: r.variant } : {}),
      }
    }
  }
  return { family: 'catppuccin', mode: 'auto' }
}

/** Parses a saved layout: the theme is always a `ThemeChoice` afterwards. */
export function parseSettings<T extends { theme?: unknown }>(
  raw: T,
): Omit<T, 'theme'> & {
  theme: ThemeChoice
} {
  return { ...raw, theme: normalizeTheme(raw.theme) }
}

export const DEFAULT_PRESET = 'Family hub'

export function defaultSettings(): DashboardSettings {
  return structuredClone(PRESETS[DEFAULT_PRESET]!)
}

const LEFT = new Set(['clock', 'weather', 'members', 'now'])

/**
 * Landscape: 12-column grid, left 4 columns hold clock/weather/members/now,
 * right 8 hold agenda/week/member_tasks/routine. Portrait: full-width rows in panel order.
 * Unknown panel kinds are dropped here but remain in `settings.panels`.
 */
export function resolveLayout(
  settings: DashboardSettings,
  o: Orientation = 'landscape',
): GridArea[] {
  const known = settings.panels.filter((p) => KNOWN_PANELS.includes(p.kind))
  if (o === 'portrait') {
    return known.map((panel, i) => ({ panel, column: '1 / 13', row: `${i + 1} / ${i + 2}` }))
  }
  const left = known.filter((p) => LEFT.has(p.kind))
  const right = known.filter((p) => !LEFT.has(p.kind))
  const out: GridArea[] = []
  left.forEach((panel, i) => out.push({ panel, column: '1 / 5', row: `${i + 1} / ${i + 2}` }))
  const span = Math.max(left.length, 1)
  right.forEach((panel, i) => {
    const rows = Math.max(1, Math.floor(span / right.length))
    const start = i * rows + 1
    const end = i === right.length - 1 ? span + 1 : start + rows
    out.push({ panel, column: '5 / 13', row: `${start} / ${end}` })
  })
  return out
}

/** A few-pixel shift that changes every 10 minutes (burn-in protection). */
export function burnInOffset(now: Date): { x: number; y: number } {
  const slot = Math.floor(now.getTime() / 600_000)
  const steps = [
    [0, 0],
    [3, 1],
    [-2, 3],
    [1, -3],
    [-3, -1],
    [2, 2],
  ] as const
  const [x, y] = steps[((slot % steps.length) + steps.length) % steps.length]!
  return { x, y }
}
