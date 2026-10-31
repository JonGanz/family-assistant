import { FAMILIES, familyOf, resolveVariant, type ThemeChoice, type ThemeVariant } from '../themes'
import { KNOWN_PANELS, normalizeTheme, PRESETS, type PanelConfig } from './layout'
import type { DashboardSettings } from './types'

type Panels = DashboardSettings['panels']

export const MAX_PANELS = 12
export const MIN_DAYS = 1
export const MAX_DAYS = 14

export const clone = (s: DashboardSettings): DashboardSettings => structuredClone(s)

export function addPanel(s: DashboardSettings, kind: string): DashboardSettings {
  if (!KNOWN_PANELS.includes(kind) || s.panels.length >= MAX_PANELS) return s
  const panel: PanelConfig = kind === 'weather' ? { kind, hourly: true } : { kind }
  return { ...s, panels: [...s.panels, panel] as Panels }
}

export function removePanel(s: DashboardSettings, index: number): DashboardSettings {
  return { ...s, panels: s.panels.filter((_, i) => i !== index) }
}

export function movePanel(s: DashboardSettings, index: number, delta: -1 | 1): DashboardSettings {
  const to = index + delta
  if (to < 0 || to >= s.panels.length) return s
  const panels = [...s.panels]
  const [p] = panels.splice(index, 1)
  panels.splice(to, 0, p!)
  return { ...s, panels }
}

export function setPanelOption(
  s: DashboardSettings,
  index: number,
  key: string,
  value: unknown,
): DashboardSettings {
  return {
    ...s,
    panels: s.panels.map((p, i) => (i === index ? { ...p, [key]: value } : p)) as Panels,
  }
}

/** A preset replaces the panels and agenda window; theme and filter stay. */
export function applyPreset(s: DashboardSettings, name: string): DashboardSettings {
  const p = PRESETS[name]
  if (!p) return s
  return { ...s, agenda_days: p.agenda_days, panels: structuredClone(p.panels) as never }
}

export function clampDays(n: number): number {
  if (!Number.isFinite(n)) return MIN_DAYS
  return Math.min(MAX_DAYS, Math.max(MIN_DAYS, Math.round(n)))
}

/** Mirrors the server's validation so the editor never offers an invalid save. */
export function isValid(s: DashboardSettings): boolean {
  return (
    s.agenda_days >= MIN_DAYS &&
    s.agenda_days <= MAX_DAYS &&
    s.panels.length <= MAX_PANELS &&
    typeof normalizeTheme(s.theme).family === 'string'
  )
}

export function toggleMember(s: DashboardSettings, id: string, all: string[]): DashboardSettings {
  const cur = s.members === 'all' ? new Set(all) : new Set<string>(s.members)
  if (cur.has(id)) cur.delete(id)
  else cur.add(id)
  const picked = all.filter((m) => cur.has(m))
  return { ...s, members: picked.length === all.length ? 'all' : picked }
}

/** Toggle one id in a panel's id-list option (`members` or `lists`); an empty list means all. */
export function togglePanelId(
  s: DashboardSettings,
  index: number,
  key: 'members' | 'lists',
  id: string,
  all: string[],
): DashboardSettings {
  const cur: unknown = s.panels[index]?.[key]
  const set = new Set<string>(Array.isArray(cur) && cur.length ? cur.map(String) : all)
  if (set.has(id)) set.delete(id)
  else set.add(id)
  const picked = all.filter((x) => set.has(x))
  return setPanelOption(
    s,
    index,
    key,
    picked.length === all.length || picked.length === 0 ? [] : picked,
  )
}

export function panelHasId(
  p: { [k: string]: unknown },
  key: 'members' | 'lists',
  id: string,
): boolean {
  const cur = p[key]
  return !Array.isArray(cur) || cur.length === 0 || cur.includes(id)
}

/** The theme of a settings object, whether it still holds the legacy string or a choice. */
export const themeOf = (s: { theme: unknown }): ThemeChoice => normalizeTheme(s.theme)

export interface ThemeControls {
  /** False for a family with one mode: the mode control is hidden. */
  showMode: boolean
  /** The only mode of a single-mode family, for the "only has a dark/light variant" note. */
  onlyMode: 'light' | 'dark' | null
  /** The variants of the effective mode; the variant select shows only when there are several. */
  variants: ThemeVariant[]
  showVariant: boolean
  /** The variant currently in effect. */
  current: ThemeVariant
}

/** Shared by the Appearance page and the layout editor so both follow the same rules. */
export function themeControls(
  choice: ThemeChoice,
  ctx: { prefersDark: boolean; night?: boolean } = { prefersDark: false },
): ThemeControls {
  const family = familyOf(choice.family)
  const modes = new Set(family.variants.map((v) => v.mode))
  const current = resolveVariant(choice, ctx)
  const variants = family.variants.filter((v) => v.mode === current.mode)
  return {
    showMode: modes.size > 1,
    onlyMode: modes.size === 1 ? family.variants[0]!.mode : null,
    variants,
    showVariant: variants.length > 1,
    current,
  }
}

/** A new family starts at its default variant, keeping the chosen mode. */
export function withFamily(choice: ThemeChoice, family: string): ThemeChoice {
  return { family, mode: choice.mode }
}

/** A new mode drops a variant that does not belong to it. */
export function withMode(choice: ThemeChoice, mode: ThemeChoice['mode']): ThemeChoice {
  const fam = familyOf(choice.family)
  const keep =
    choice.variant && fam.variants.some((v) => v.id === choice.variant && v.mode === mode)
  return { family: choice.family, mode, ...(keep ? { variant: choice.variant } : {}) }
}

export function withVariant(choice: ThemeChoice, variant: string): ThemeChoice {
  return { ...choice, variant }
}

export const FAMILY_OPTIONS = FAMILIES.map((f) => ({ id: f.id, label: f.label }))
