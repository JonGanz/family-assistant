import { describe, expect, it } from 'vitest'
import {
  burnInOffset,
  defaultSettings,
  normalizeTheme,
  parseSettings,
  portraitRoutineOrder,
  PRESETS,
  resolveLayout,
  splitRoutineAreas,
  withMealsPanel,
} from './index'

describe('layout engine', () => {
  it('has the presets, default is Family hub', () => {
    expect(Object.keys(PRESETS)).toEqual(['Family hub', 'Chores board', 'Calendar wall', 'Minimal'])
    expect(defaultSettings().panels.map((p) => p.kind)).toEqual([
      'clock',
      'weather',
      'agenda',
      'meals',
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
    expect(areas.find((a) => a.panel.kind === 'routine')!.row).toBe('3 / 4')
  })
  it('portrait stacks full width', () => {
    const areas = resolveLayout(defaultSettings(), 'portrait')
    expect(areas.map((a) => a.row)).toEqual(['1 / 2', '2 / 3', '3 / 4', '4 / 5', '5 / 6', '6 / 7'])
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

describe('withMealsPanel', () => {
  it('adds meals after the agenda of a layout saved before the panel existed', () => {
    const old = [{ kind: 'clock' }, { kind: 'agenda' }, { kind: 'now' }]
    expect(withMealsPanel(old).map((p) => p.kind)).toEqual(['clock', 'agenda', 'meals', 'now'])
  })
  it('keeps layouts that have meals or no agenda', () => {
    const has = [{ kind: 'agenda' }, { kind: 'meals' }]
    expect(withMealsPanel(has)).toBe(has)
    const chores = [{ kind: 'clock' }, { kind: 'member_tasks' }]
    expect(withMealsPanel(chores)).toBe(chores)
  })
})

describe('per-run routine cards', () => {
  const area = (kind: string, run?: string) => ({
    panel: run ? { kind, run } : { kind },
    column: '5 / 13',
    row: '1 / 2',
  })
  it('splits a routine area into one area per run, once per run', () => {
    const out = splitRoutineAreas([area('agenda'), area('routine'), area('routine')], () => [
      'a',
      'b',
    ])
    expect(out.map((x) => [x.panel.kind, x.panel['run']])).toEqual([
      ['agenda', undefined],
      ['routine', 'a'],
      ['routine', 'b'],
    ])
    expect(out[1]!.column).toBe('5 / 13')
  })
  it('keeps an area that already names its run, and drops a routine with no runs', () => {
    expect(splitRoutineAreas([area('routine', 'x')], () => ['a'])).toHaveLength(1)
    expect(splitRoutineAreas([area('routine')], () => [])).toHaveLength(0)
  })
  it('orders portrait cards bottom-up: left then right, then the next row above', () => {
    expect(portraitRoutineOrder(['1'])).toEqual(['1'])
    expect(portraitRoutineOrder(['1', '2'])).toEqual(['1', '2'])
    // DOM order is top to bottom: 3 sits above 1, 4 above 2.
    expect(portraitRoutineOrder(['1', '2', '3', '4'])).toEqual(['3', '4', '1', '2'])
    expect(portraitRoutineOrder(['1', '2', '3'])).toEqual(['3', '1', '2'])
    expect(portraitRoutineOrder(['1', '2', '3', '4', '5'])).toEqual(['5', '3', '4', '1', '2'])
  })
})
