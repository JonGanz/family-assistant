import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DashboardCanvas from './DashboardCanvas.vue'
import ConfettiLayer from './effects/ConfettiLayer.vue'
import SnowLayer from './effects/SnowLayer.vue'
import { SNOW_PARAMS, snowMode } from './effects/snowModel'
import RoutinePanel from './panels/RoutinePanel.vue'
import {
  FIRED_KEY,
  hasFired,
  markFired,
  memberShades,
  newlyCompleted,
  onConfetti,
  runKey,
  shades,
  type ConfettiRequest,
} from './effects/confettiModel'
import type { DashboardData, DashboardSettings } from './types'

const NOW = new Date('2026-10-24T10:00:00Z')
const settings = (over: Partial<DashboardSettings> = {}): DashboardSettings =>
  ({
    theme: { family: 'catppuccin', mode: 'light' },
    agenda_days: 2,
    members: 'all',
    panels: [{ kind: 'agenda' }],
    ...over,
  }) as DashboardSettings

const run = (done: boolean, date = '2026-10-24') => ({
  id: 'r1',
  member: 'm1',
  routine: 'x',
  routine_name: 'Morning',
  run_date: date,
  starts_at: '06:00',
  done_by: '08:00',
  status: 'active',
  done: 0,
  total: 2,
  steps: [
    { step: 's1', text: 'One', done_at: '2026-10-24T06:00:00Z' },
    { step: 's2', text: 'Two', done_at: done ? '2026-10-24T06:01:00Z' : null },
  ],
})
const data = (runs: unknown[], over: Partial<DashboardSettings> = {}): DashboardData =>
  ({
    household: { name: 'Example Household', tz: 'UTC', units: 'metric', location: null },
    members: [{ id: 'm1', display_name: 'Sam', color: 'green' }],
    events: [],
    advisories: [],
    tasks: [],
    routines: runs,
    weather: null,
    settings: settings(over),
  }) as unknown as DashboardData

beforeEach(() => localStorage.clear())

describe('confetti model', () => {
  it('fires on the transition to done, once, and not for runs already done at load', () => {
    const prev = new Map<string, boolean>()
    const k = runKey({ id: 'r1', run_date: '2026-10-24' })
    expect(newlyCompleted(prev, [{ key: k, runDate: '2026-10-24', done: false }])).toEqual([])
    const fire = newlyCompleted(prev, [{ key: k, runDate: '2026-10-24', done: true }])
    expect(fire).toHaveLength(1)
    markFired(k, '2026-10-24', '2026-10-24')
    // uncheck then recheck
    newlyCompleted(prev, [{ key: k, runDate: '2026-10-24', done: false }])
    expect(newlyCompleted(prev, [{ key: k, runDate: '2026-10-24', done: true }])).toEqual([])
    // already done at load
    const p2 = new Map<string, boolean>()
    expect(
      newlyCompleted(p2, [{ key: 'z@2026-10-24', runDate: '2026-10-24', done: true }]),
    ).toEqual([])
  })
  it('fires for the next day run and prunes old keys', () => {
    markFired('old@2026-10-01', '2026-10-01', '2026-10-24')
    markFired('r1@2026-10-24', '2026-10-24', '2026-10-24')
    expect(hasFired('old@2026-10-01')).toBe(false)
    expect(hasFired('r1@2026-10-24')).toBe(true)
    expect(hasFired('r1@2026-10-25')).toBe(false)
    expect(Object.keys(JSON.parse(localStorage.getItem(FIRED_KEY)!))).toEqual(['r1@2026-10-24'])
  })
  it('survives broken storage', () => {
    localStorage.setItem(FIRED_KEY, '{nope')
    expect(hasFired('a')).toBe(false)
    expect(() => markFired('a', '2026-10-24', '2026-10-24')).not.toThrow()
  })
  it('derives lighter and darker shades of the member colour', () => {
    const s = shades('#336699')
    expect(new Set(s).size).toBe(5)
    expect(s.some((x) => x.includes('white'))).toBe(true)
    expect(s.some((x) => x.includes('black'))).toBe(true)
    expect(memberShades('green')[2]).toBe('var(--hue-green)')
    expect(memberShades(null)[2]).toBe('var(--hue-blue)')
  })
})

describe('routine panel confetti', () => {
  const mk = (d: DashboardData) =>
    mount(RoutinePanel, {
      props: { data: d, settings: d.settings, panel: { kind: 'routine', pinned: true }, now: NOW },
    })
  it('fires once when the last step completes, with the member shades', async () => {
    const got: ConfettiRequest[] = []
    const off = onConfetti((r) => got.push(r))
    const w = mk(data([run(false)]))
    expect(got).toHaveLength(0)
    await w.setProps({ data: data([run(true)]), settings: settings() })
    expect(got).toHaveLength(1)
    expect(got[0]!.colors[2]).toBe('var(--hue-green)')
    await w.setProps({ data: data([run(false)]) })
    await w.setProps({ data: data([run(true)]) })
    expect(got).toHaveLength(1)
    await w.setProps({ data: data([run(false, '2026-10-25')]) })
    await w.setProps({ data: data([run(true, '2026-10-25')]) })
    expect(got).toHaveLength(2)
    off()
  })
  it('does not fire for a run already done at load, or when switched off', async () => {
    const got: ConfettiRequest[] = []
    const off = onConfetti((r) => got.push(r))
    mk(data([run(true)]))
    expect(got).toHaveLength(0)
    const d = data([run(false)], { confetti: false })
    const w = mk(d)
    await w.setProps({
      data: data([run(true)], { confetti: false }),
      settings: settings({ confetti: false }),
    })
    expect(got).toHaveLength(0)
    off()
  })
})

const wx = (d: DashboardData, code?: number): DashboardData =>
  code === undefined
    ? d
    : ({ ...d, weather: { current: { weather_code: code } } } as unknown as DashboardData)

const mount2 = (over: Partial<DashboardSettings> = {}, code?: number) =>
  mount(DashboardCanvas, {
    props: { data: wx(data([], over), code), now: NOW },
    global: { plugins: [createPinia()] },
  })

describe('snow model', () => {
  it('maps WMO codes to modes', () => {
    expect([71, 77].map(snowMode)).toEqual(['light', 'light'])
    expect(snowMode(73)).toBe('moderate')
    expect(snowMode(75)).toBe('heavy')
    expect([85, 86].map(snowMode)).toEqual(['flurries', 'flurries'])
    for (const c of [0, 3, 61, 95, undefined]) expect(snowMode(c)).toBe('none')
  })
  it('flurries are denser, faster and capped higher than heavy', () => {
    const f = SNOW_PARAMS.flurries
    const h = SNOW_PARAMS.heavy
    expect(f.density).toBeGreaterThan(h.density)
    expect(f.speed).toBeGreaterThan(h.speed)
    expect(f.max).toBeGreaterThan(h.max)
    expect(f.max).toBeLessThanOrEqual(300)
  })
})

describe('snow layer on the canvas', () => {
  const layer = '[data-testid="snow-layer"]'
  it('shows only when the weather has snow', () => {
    expect(mount2({}, 0).find(layer).exists()).toBe(false)
    expect(mount2().find(layer).exists()).toBe(false)
    expect(mount2({}, 73).find(layer).exists()).toBe(true)
  })
  it('passes flurries for snow showers', () => {
    const w = mount2({}, 85)
    expect(w.findComponent(SnowLayer).props('mode')).toBe('flurries')
  })
  it('the switch hides it even when snowing', () => {
    expect(mount2({ snow: false }, 75).find(layer).exists()).toBe(false)
  })
  it('follows live weather changes', async () => {
    const w = mount2({}, 73)
    await w.setProps({ data: wx(data([]), 0) })
    expect(w.find(layer).exists()).toBe(false)
  })
})

describe('effect layers', () => {
  it('are on by default and each toggle turns its layer off', () => {
    const on = mount2()
    expect(on.find('[data-testid="confetti-layer"]').exists()).toBe(true)
    expect(mount2({ confetti: false }).find('[data-testid="confetti-layer"]').exists()).toBe(false)
  })
  it('are aria-hidden, non-interactive and do not crash without a canvas context', () => {
    const getContext = vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null)
    const snow = mount(SnowLayer, { props: { mode: 'moderate' } })
    const conf = mount(ConfettiLayer)
    for (const w of [snow, conf]) {
      expect(w.attributes('aria-hidden')).toBe('true')
      expect(w.attributes('tabindex') ?? '-1').toBe('-1')
    }
    getContext.mockRestore()
  })
})
