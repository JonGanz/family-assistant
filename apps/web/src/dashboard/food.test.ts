import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../api/food', async (orig) => {
  const real = await orig<typeof import('../api/food')>()
  const kiosk = {
    grocery: vi.fn().mockResolvedValue({
      aisles: [],
      stores: [],
      staples: [],
      items: [
        {
          id: 'g1',
          text: 'Milk',
          display: 'Milk',
          section: 'list',
          manual: true,
          sources: [],
          version: 1,
          checked_at: null,
        },
      ],
    }),
    plan: vi.fn().mockResolvedValue([]),
    slots: vi.fn().mockResolvedValue([]),
  }
  return { ...real, foodKiosk: kiosk, food: kiosk }
})

import DashboardCanvas from './DashboardCanvas.vue'
import DayCell from './DayCell.vue'
import { addPanel } from './editor'
import { defaultSettings, KNOWN_PANELS, PRESETS, resolveLayout } from './layout'
import { mealsPanelVisible, sortedMeals } from './mealsModel'
import PanelHost from './panels/PanelHost.vue'
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
const data = (over: Partial<DashboardData> = {}): DashboardData =>
  ({
    household: { name: 'Example Household', tz: 'UTC', units: 'metric', location: null },
    members: [],
    events: [],
    advisories: [],
    tasks: [],
    routines: [],
    weather: null,
    settings: settings(),
    ...over,
  }) as DashboardData

const meals = [
  { slot_name: 'Dinner', time: '18:30', title: 'Lentil soup' },
  { slot_name: 'Breakfast', time: '07:30', title: 'Porridge' },
]

beforeEach(() => setActivePinia(createPinia()))

describe('food view button', () => {
  const mk = (food_view?: boolean) =>
    mount(DashboardCanvas, {
      props: { data: data({ settings: settings({ food_view }) }), now: NOW },
      global: { plugins: [createPinia()] },
    })
  it('shows by default and when on', () => {
    expect(mk().find('[data-testid="kiosk-view-food"]').exists()).toBe(true)
    expect(mk(true).find('[data-testid="kiosk-view-food"]').exists()).toBe(true)
  })
  it('is hidden when food_view is false', () => {
    expect(mk(false).find('[data-testid="kiosk-view-food"]').exists()).toBe(false)
  })
  it('sits between Week and Whiteboard', () => {
    const ids = mk()
      .findAll('nav button')
      .map((b) => b.attributes('data-view'))
    expect(ids).toEqual(['home', 'week', 'food', 'whiteboard'])
  })
})

describe('meals panel', () => {
  it('orders by slot and shows time and title', () => {
    expect(sortedMeals({ meals }).map((m) => m.slot_name)).toEqual(['Breakfast', 'Dinner'])
    const w = mount(PanelHost, {
      props: { data: data({ meals }), settings: settings(), panel: { kind: 'meals' }, now: NOW },
    })
    const rows = w.findAll('[data-testid="meal-row"]').map((r) => r.text())
    expect(rows[0]).toContain('Breakfast')
    expect(rows[0]).toContain('07:30')
    expect(rows[1]).toContain('Lentil soup')
  })
  it('hides when empty, unless it is the only panel in its column', () => {
    const meal = { kind: 'meals' }
    const agenda = { kind: 'agenda' }
    const clock = { kind: 'clock' }
    const d = data({ meals: [] })
    expect(mealsPanelVisible(meal, [clock, agenda, meal], d, 'landscape')).toBe(false)
    expect(mealsPanelVisible(meal, [clock, meal], d, 'landscape')).toBe(true)
    expect(mealsPanelVisible(meal, [clock, meal], d, 'portrait')).toBe(true)
    expect(mealsPanelVisible(meal, [agenda, meal], d, 'portrait')).toBe(false)
    expect(mealsPanelVisible(meal, [agenda, meal], data({ meals }), 'portrait')).toBe(true)
  })
  it('the canvas drops an empty meals panel beside others', () => {
    const s = settings({ panels: [{ kind: 'agenda' }, { kind: 'meals' }] as never })
    const w = mount(DashboardCanvas, {
      props: { data: data({ settings: s, meals: [] }), now: NOW },
      global: { plugins: [createPinia()] },
    })
    expect(w.find('[data-testid="panel-meals"]').exists()).toBe(false)
    const w2 = mount(DashboardCanvas, {
      props: { data: data({ settings: s, meals }), now: NOW },
      global: { plugins: [createPinia()] },
    })
    expect(w2.find('[data-testid="panel-meals"]').exists()).toBe(true)
  })
})

describe('layout', () => {
  it('the Family hub preset has meals after the agenda', () => {
    const kinds = PRESETS['Family hub']!.panels.map((p) => p.kind)
    expect(kinds.indexOf('meals')).toBe(kinds.indexOf('agenda') + 1)
    expect(KNOWN_PANELS).toContain('meals')
    expect(defaultSettings().panels.some((p) => p.kind === 'meals')).toBe(true)
  })
  it('an older saved layout without meals still loads', () => {
    const old = settings({ panels: [{ kind: 'clock' }, { kind: 'agenda' }] as never })
    expect(resolveLayout(old as never).map((a) => a.panel.kind)).toEqual(['clock', 'agenda'])
    const w = mount(DashboardCanvas, {
      props: { data: data({ settings: old }), now: NOW },
      global: { plugins: [createPinia()] },
    })
    expect(w.find('[data-testid="panel-meals"]').exists()).toBe(false)
    expect(addPanel(old, 'meals').panels.at(-1)).toEqual({ kind: 'meals' })
  })
})

describe('week view meals', () => {
  it('a day cell shows the meals line first', () => {
    const w = mount(DayCell, {
      props: {
        events: [],
        tasks: [],
        meals: ['Dinner: Lentil soup'],
        tz: 'UTC',
        stripe: {},
        label: 'x',
        color: '#000',
      },
    })
    expect(w.find('[data-testid="meal-line"]').text()).toContain('Dinner: Lentil soup')
  })
  it('the week table passes the day meals to the first column', () => {
    const w = mount(PanelHost, {
      props: {
        data: data({
          members: [{ id: 'm1', display_name: 'Alex', role: 'adult', color: '#3366cc' }] as never,
        }),
        settings: settings(),
        panel: { kind: 'week_table', start: '2026-10-24' },
        now: NOW,
        meals: [{ date: '2026-10-24', slot: 'Dinner', title: 'Lentil soup' }],
      },
    })
    expect(w.find('tr[data-day="2026-10-24"]').text()).toContain('Dinner: Lentil soup')
    expect(w.find('tr[data-day="2026-10-25"]').text()).not.toContain('Lentil')
  })
})

describe('kiosk shopping list', () => {
  async function open(elevated: boolean) {
    const w = mount(DashboardCanvas, {
      props: { data: data({ elevated }), now: NOW },
      global: { plugins: [createPinia()] },
    })
    await w.find('[data-testid="kiosk-view-food"]').trigger('click')
    await w.find('[data-testid="food-tab-shopping"]').trigger('click')
    await flushPromises()
    return w
  }
  it('shows delete only when elevated', async () => {
    expect((await open(false)).find('[aria-label="Delete Milk"]').exists()).toBe(false)
    expect((await open(true)).find('[aria-label="Delete Milk"]').exists()).toBe(true)
  })
  it('offers the add field without elevation', async () => {
    expect((await open(false)).find('[data-testid="grocery-add-input"]').exists()).toBe(true)
  })
})
