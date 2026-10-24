export type PanelKind = 'clock' | 'weather' | 'advisories' | 'agenda' | 'week' | 'members'
export type Orientation = 'landscape' | 'portrait'

export interface PanelConfig {
  kind: string
  [key: string]: unknown
}

export interface DashboardSettings {
  orientation: Orientation
  theme: 'auto' | 'light' | 'dark'
  agenda_days: number
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
  'advisories',
  'agenda',
  'week',
  'members',
]

const base = { orientation: 'landscape', theme: 'auto', members: 'all' } as const

export const PRESETS: Record<string, DashboardSettings> = {
  'Family hub': {
    ...base,
    agenda_days: 3,
    panels: [
      { kind: 'clock' },
      { kind: 'weather', hourly: true },
      { kind: 'advisories' },
      { kind: 'agenda' },
    ],
  },
  'Calendar wall': {
    ...base,
    agenda_days: 7,
    panels: [
      { kind: 'clock' },
      { kind: 'weather', hourly: false },
      { kind: 'advisories' },
      { kind: 'week' },
    ],
  },
  Minimal: { ...base, agenda_days: 2, panels: [{ kind: 'clock' }, { kind: 'agenda' }] },
}

export const DEFAULT_PRESET = 'Family hub'

export function defaultSettings(): DashboardSettings {
  return structuredClone(PRESETS[DEFAULT_PRESET]!)
}

const LEFT = new Set(['clock', 'weather', 'advisories', 'members'])

/**
 * Landscape: 12-column grid, left 4 columns hold clock/weather/advisories/members,
 * right 8 hold agenda/week. Portrait: full-width rows in panel order.
 * Unknown panel kinds are dropped here but remain in `settings.panels`.
 */
export function resolveLayout(settings: DashboardSettings, orientation?: Orientation): GridArea[] {
  const o = orientation ?? settings.orientation
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
