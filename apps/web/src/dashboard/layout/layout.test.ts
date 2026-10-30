import { describe, expect, it } from 'vitest'
import {
  burnInOffset,
  defaultSettings,
  normalizeTheme,
  parseSettings,
  PRESETS,
  resolveLayout,
} from './index'

describe('layout engine', () => {
  it('has the presets, default is Family hub', () => {
    expect(Object.keys(PRESETS)).toEqual(['Family hub', 'Chores board', 'Calendar wall', 'Minimal'])
    expect(defaultSettings().panels.map((p) => p.kind)).toEqual([
      'clock',
      'weather',
      'agenda',
      'now',
      'routine',
    ])
  })
  it('Chores board has member_tasks', () => {
    expect(PRESETS['Chores board']!.panels.map((p) => p.kind)).toContain('member_tasks')
  })
  it('landscape: left 4 columns and right 8', () => {
    const areas = resolveLayout(defaultSettings(), 'landscape')
    const col = (k: string) => areas.find((a) => a.panel.kind === k)!.column
    expect(col('clock')).toBe('1 / 5')
    expect(col('weather')).toBe('1 / 5')
    expect(col('agenda')).toBe('5 / 13')
    expect(col('now')).toBe('1 / 5')
    expect(col('routine')).toBe('5 / 13')
    expect(areas.find((a) => a.panel.kind === 'agenda')!.row).toBe('1 / 2')
    expect(areas.find((a) => a.panel.kind === 'routine')!.row).toBe('2 / 4')
  })
  it('portrait stacks full width', () => {
    const areas = resolveLayout(defaultSettings(), 'portrait')
    expect(areas.map((a) => a.row)).toEqual(['1 / 2', '2 / 3', '3 / 4', '4 / 5', '5 / 6'])
    expect(areas.every((a) => a.column === '1 / 13')).toBe(true)
  })
  it('drops unknown panels from the grid but keeps them in the data', () => {
    const s = defaultSettings()
    s.panels.push({ kind: 'hologram' })
    expect(resolveLayout(s, 'landscape').some((a) => a.panel.kind === 'hologram')).toBe(false)
    expect(s.panels.some((p) => p.kind === 'hologram')).toBe(true)
  })
  it('ignores a stored inbox panel (D-068)', () => {
    const s = { ...defaultSettings(), panels: [{ kind: 'clock' }, { kind: 'inbox' }] }
    expect(resolveLayout(s).map((a) => a.panel.kind)).toEqual(['clock'])
  })

  it('places member_tasks on the right', () => {
    const areas = resolveLayout(PRESETS['Chores board']!, 'landscape')
    expect(areas.find((a) => a.panel.kind === 'member_tasks')!.column).toBe('5 / 13')
  })
  it('burn-in offset is small and changes every 10 minutes', () => {
    const t = new Date('2026-10-24T10:00:00Z')
    const a = burnInOffset(t)
    expect(burnInOffset(new Date(t.getTime() + 5 * 60_000))).toEqual(a)
    expect(burnInOffset(new Date(t.getTime() + 10 * 60_000))).not.toEqual(a)
    expect(Math.abs(a.x)).toBeLessThanOrEqual(3)
  })
})

describe('saved theme', () => {
  it('reads the legacy string as the Catppuccin choice', () => {
    expect(parseSettings({ ...defaultSettings(), theme: 'dark' as never }).theme).toEqual({
      family: 'catppuccin',
      mode: 'dark',
    })
  })
  it('keeps an object, unknown families included', () => {
    expect(normalizeTheme({ family: 'catppuccin', mode: 'light', variant: 'latte' })).toEqual({
      family: 'catppuccin',
      mode: 'light',
      variant: 'latte',
    })
    expect(normalizeTheme({ family: 'nope', mode: 'dark' })).toEqual({
      family: 'nope',
      mode: 'dark',
    })
    expect(normalizeTheme(undefined)).toEqual({ family: 'catppuccin', mode: 'auto' })
  })
  it('presets keep an object theme', () => {
    for (const p of Object.values(PRESETS))
      expect(p.theme).toEqual({ family: 'catppuccin', mode: 'auto' })
  })
})
