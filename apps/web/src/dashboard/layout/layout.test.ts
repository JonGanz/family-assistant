import { describe, expect, it } from 'vitest'
import { burnInOffset, defaultSettings, PRESETS, resolveLayout } from './index'

describe('layout engine', () => {
  it('has the three presets, default is Family hub', () => {
    expect(Object.keys(PRESETS)).toEqual(['Family hub', 'Calendar wall', 'Minimal'])
    expect(defaultSettings().panels.map((p) => p.kind)).toEqual([
      'clock',
      'weather',
      'advisories',
      'agenda',
    ])
  })
  it('landscape: left 4 columns and right 8', () => {
    const areas = resolveLayout(defaultSettings(), 'landscape')
    const col = (k: string) => areas.find((a) => a.panel.kind === k)!.column
    expect(col('clock')).toBe('1 / 5')
    expect(col('weather')).toBe('1 / 5')
    expect(col('agenda')).toBe('5 / 13')
    expect(areas.find((a) => a.panel.kind === 'agenda')!.row).toBe('1 / 4')
  })
  it('portrait stacks full width', () => {
    const areas = resolveLayout(defaultSettings(), 'portrait')
    expect(areas.map((a) => a.row)).toEqual(['1 / 2', '2 / 3', '3 / 4', '4 / 5'])
    expect(areas.every((a) => a.column === '1 / 13')).toBe(true)
  })
  it('drops unknown panels from the grid but keeps them in the data', () => {
    const s = defaultSettings()
    s.panels.push({ kind: 'hologram' })
    expect(resolveLayout(s, 'landscape').some((a) => a.panel.kind === 'hologram')).toBe(false)
    expect(s.panels.some((p) => p.kind === 'hologram')).toBe(true)
  })
  it('burn-in offset is small and changes every 10 minutes', () => {
    const t = new Date('2026-10-24T10:00:00Z')
    const a = burnInOffset(t)
    expect(burnInOffset(new Date(t.getTime() + 5 * 60_000))).toEqual(a)
    expect(burnInOffset(new Date(t.getTime() + 10 * 60_000))).not.toEqual(a)
    expect(Math.abs(a.x)).toBeLessThanOrEqual(3)
  })
})
