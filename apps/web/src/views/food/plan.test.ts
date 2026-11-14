import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const h = vi.hoisted(() => ({
  push: vi.fn(),
  path: { value: '/food/plan' },
  api: {
    plan: vi.fn(),
    addEntry: vi.fn(),
    reconcile: vi.fn(),
    recipes: vi.fn(),
    recipe: vi.fn(),
    previewTemplate: vi.fn(),
    templates: vi.fn(),
    templateFromPlan: vi.fn(),
    recipeImageUrl: (id: string) => `/img/${id}`,
  },
}))
vi.mock('vue-router', () => ({
  useRoute: () => ({ path: h.path.value, query: {}, params: {} }),
  useRouter: () => ({ push: h.push, replace: vi.fn() }),
  onBeforeRouteLeave: vi.fn(),
  RouterLink: { template: '<a><slot /></a>' },
}))
vi.mock('../../api/food', () => ({ food: h.api, foodKiosk: h.api }))

import ApplyTemplateSheet from '../../components/food/plan/ApplyTemplateSheet.vue'
import FoodTabs from '../../components/food/FoodTabs.vue'
import ImpactDialog from '../../components/food/plan/ImpactDialog.vue'
import MealPlanGrid from '../../components/food/plan/MealPlanGrid.vue'
import SaveAsTemplateSheet from '../../components/food/plan/SaveAsTemplateSheet.vue'
import { useHouseholdStore } from '../../stores/household'
import { useMealPlanStore } from '../../stores/mealPlan'
import { useMealsSettingsStore } from '../../stores/mealsSettings'
import { useSessionStore } from '../../stores/session'
import { resetSnackbar } from '../../ui/composables/useSnackbar'
import MealPlanView from './MealPlanView.vue'
import { agendaRange } from './planAgendaRange'

const TODAY = '2026-11-09' // a Monday
const SLOTS = [
  { id: 's-b', name: 'Breakfast', time: '07:30', visible: false, ord: 0 },
  { id: 's-d', name: 'Dinner', time: '18:00', visible: true, ord: 2 },
]
const entry = (id: string, ord: number, over: Record<string, unknown> = {}) => ({
  id,
  date: TODAY,
  slot_id: 's-d',
  ord,
  title: `Meal ${id}`,
  free_text: `Meal ${id}`,
  recipe_id: null,
  servings: null,
  version: 1,
  ...over,
})

function asUser(role: 'adult' | 'child') {
  const s = useSessionStore()
  s.me = { actor: { kind: 'user', member: { id: 'm1', role } }, household: { tz: 'UTC' } } as never
}
function ready() {
  const settings = useMealsSettingsStore()
  settings.slots = SLOTS as never
  settings.settings = { meal_plan_horizon_weeks: 2 } as never
  settings.loaded = true
  const household = useHouseholdStore()
  household.view = { tz: 'UTC', week_start: 'monday' } as never
  household.loaded = true
}
const q = (id: string) => document.querySelector<HTMLElement>(`[data-testid="${id}"]`)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  h.path.value = '/food/plan'
  h.api.plan.mockResolvedValue([])
  h.api.templates.mockResolvedValue([])
  h.api.recipes.mockResolvedValue({ items: [] })
  resetSnackbar()
  localStorage.clear()
  document.body.innerHTML = ''
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(`${TODAY}T12:00:00Z`))
})
afterEach(() => vi.useRealTimers())

describe('MealPlanGrid', () => {
  it('starts on the given day and inverts today', async () => {
    ready()
    const w = mount(MealPlanGrid, { props: { from: '2026-11-02', weeks: 2, editable: false } })
    await flushPromises()
    const rows = w.findAll('.week')
    expect(rows).toHaveLength(2)
    expect(rows[0]!.attributes('data-week')).toBe('2026-11-02')
    const days = rows[0]!.findAll('.day').map((d) => d.attributes('data-day'))
    expect(days[0]).toBe('2026-11-02')
    expect(days).toHaveLength(7)
    const today = w.findAll('.day.is-today')
    expect(today).toHaveLength(1)
    expect(today[0]!.attributes('data-day')).toBe(TODAY)
    expect(h.api.plan).toHaveBeenCalledWith('2026-11-02', '2026-11-15')
  })

  it('draws only the visible slots', async () => {
    ready()
    const w = mount(MealPlanGrid, { props: { from: '2026-11-02', weeks: 2, editable: false } })
    await flushPromises()
    expect(w.find(`[data-testid="plan-cell-${TODAY}-dinner"]`).exists()).toBe(true)
    expect(w.find(`[data-testid="plan-cell-${TODAY}-breakfast"]`).exists()).toBe(false)
  })

  it('lists a slot with two entries in order', async () => {
    ready()
    h.api.plan.mockResolvedValue([entry('b', 2), entry('a', 1)])
    const w = mount(MealPlanGrid, { props: { from: '2026-11-02', weeks: 2, editable: false } })
    await flushPromises()
    const chips = w.findAll(`[data-testid="plan-cell-${TODAY}-dinner"] .chip__title`)
    expect(chips.map((c) => c.text())).toEqual(['Meal a', 'Meal b'])
  })
})

describe('FoodTabs', () => {
  it('remembers the tab and shows Templates to adults only', async () => {
    asUser('adult')
    h.path.value = '/food/recipes'
    const w = mount(FoodTabs, { attachTo: document.body })
    expect(localStorage.getItem('fa.food.tab')).toBe('recipes')
    expect(w.find('[data-testid="food-tab-templates"]').exists()).toBe(true)
    await w.find('[data-testid="food-tab-list"]').trigger('click')
    expect(h.push).toHaveBeenCalledWith('/food/list')
    expect(localStorage.getItem('fa.food.tab')).toBe('list')
    w.unmount()

    setActivePinia(createPinia())
    asUser('child')
    const c = mount(FoodTabs)
    expect(c.find('[data-testid="food-tab-templates"]').exists()).toBe(false)
    expect(c.find('[data-testid="food-tab-plan"]').exists()).toBe(true)
  })
})

describe('ImpactDialog and the plan view', () => {
  const impact = [
    {
      item_id: 'i1',
      entry_id: 'e1',
      text: 'Flour',
      from_qty: 500,
      to_qty: 250,
      unit: 'g',
      remove: false,
    },
    { item_id: 'i2', entry_id: 'e1', text: 'Basil', remove: true },
  ]

  it('lists the affected items and offers both answers', async () => {
    const w = mount(ImpactDialog, { props: { items: impact as never }, attachTo: document.body })
    await flushPromises()
    expect(q('impact-dialog')?.textContent).toContain('Flour')
    expect(q('impact-dialog')?.textContent).toContain('removed')
    q('impact-update')!.click()
    q('impact-leave')!.click()
    expect(w.emitted('update')).toHaveLength(1)
    expect(w.emitted('leave')).toHaveLength(1)
  })

  it('shows the dialog after a write and reconciles on "Update the list"', async () => {
    asUser('adult')
    ready()
    h.api.addEntry.mockResolvedValue({ entries: [], change_id: null, grocery_impact: impact })
    h.api.reconcile.mockResolvedValue({ change_id: 'c1', changed: 2 })
    const w = mount(MealPlanView, { attachTo: document.body })
    await flushPromises()
    q('plan-add')!.click()
    await flushPromises()
    // Free text: the second segment.
    ;[...document.querySelectorAll<HTMLElement>('[role=radio]')]
      .find((b) => b.textContent?.includes('Free text'))!
      .click()
    await flushPromises()
    const input = q('pick-text') as HTMLInputElement
    input.value = 'Pancakes'
    input.dispatchEvent(new Event('input'))
    await flushPromises()
    q('entry-save')!.click()
    await flushPromises()
    expect(h.api.addEntry).toHaveBeenCalledWith({
      date: TODAY,
      slot_id: 's-d',
      free_text: 'Pancakes',
    })
    expect(q('impact-dialog')).not.toBeNull()
    q('impact-update')!.click()
    await flushPromises()
    expect(h.api.reconcile).toHaveBeenCalledWith({ entry_ids: ['e1'] })
    expect(useMealPlanStore().impact).toEqual([])
    w.unmount()
  })

  it('gives a child no edit controls', async () => {
    asUser('child')
    ready()
    h.api.plan.mockResolvedValue([entry('a', 1)])
    const w = mount(MealPlanView, { attachTo: document.body })
    await flushPromises()
    expect(w.findAll('.chip')).toHaveLength(1)
    expect(q('plan-add')).toBeNull()
    expect(q('plan-apply-template')).toBeNull()
    expect(q('plan-make-list')).toBeNull()
    expect(w.find('.slot__add').exists()).toBe(false)
    expect(w.find('.chip').attributes('draggable')).not.toBe('true')
    w.unmount()
  })
})

describe('ApplyTemplateSheet', () => {
  it('preselects Add for slots with free text and Replace otherwise', async () => {
    ready()
    h.api.templates.mockResolvedValue([
      { id: 't1', name: 'Week A', days: 7, slot_ids: ['s-d'], version: 1 },
    ])
    const target = (date: string, existing: unknown[]) => ({
      date,
      day: 1,
      slot_id: 's-d',
      slot_visible: true,
      items: [{ day: 1, slot_id: 's-d', title: 'Soup', free_text: null, recipe_id: 'r1' }],
      existing,
    })
    h.api.previewTemplate.mockResolvedValue({
      targets: [
        target('2026-11-10', [entry('note', 1, { date: '2026-11-10' })]),
        target('2026-11-11', [entry('rec', 1, { date: '2026-11-11', recipe_id: 'r9' })]),
        target('2026-11-12', []),
      ],
    })
    const w = mount(ApplyTemplateSheet, {
      props: { modelValue: true, defaultStart: '2026-11-10', minDate: TODAY },
      attachTo: document.body,
    })
    await flushPromises()
    q('apply-preview')!.click()
    await flushPromises()
    expect(q('template-conflict-note')).not.toBeNull()
    expect(q('template-conflict-rec')).not.toBeNull()
    expect((q('template-add-note') as HTMLInputElement).checked).toBe(true)
    expect((q('template-replace-rec') as HTMLInputElement).checked).toBe(true)
    q('template-apply-confirm')!.click()
    expect(w.emitted('apply')![0]).toEqual([
      't1',
      { start: '2026-11-10', add_slots: [{ date: '2026-11-10', slot_id: 's-d' }] },
    ])
    w.unmount()
  })
})

describe('SaveAsTemplateSheet', () => {
  it('sends the range and the chosen slots', async () => {
    ready()
    const w = mount(SaveAsTemplateSheet, {
      props: { modelValue: true, defaultFrom: '2026-11-09' },
      attachTo: document.body,
    })
    await flushPromises()
    const name = q('save-template-name') as HTMLInputElement
    name.value = 'Busy week'
    name.dispatchEvent(new Event('input'))
    // Breakfast is hidden, so it starts unchecked; include it too.
    q('save-template-slot-breakfast')!.querySelector('input')!.click()
    await flushPromises()
    q('save-template-confirm')!.click()
    expect(w.emitted('save')![0]).toEqual([
      { name: 'Busy week', from: '2026-11-09', to: '2026-11-15', slot_ids: ['s-b', 's-d'] },
    ])
    w.unmount()
  })

  it('forwards the request unchanged to the API', async () => {
    const body = { name: 'X', from: '2026-11-09', to: '2026-11-15', slot_ids: ['s-d'] }
    h.api.templateFromPlan.mockResolvedValue({ id: 't9', version: 1 })
    await useMealPlanStore().fromPlan(body)
    expect(h.api.templateFromPlan).toHaveBeenCalledWith(body)
  })
})

describe('agendaRange', () => {
  it('runs from two days back to today plus the horizon', () => {
    const r = agendaRange(TODAY, 2)
    expect(r.from).toBe('2026-11-07')
    expect(r.to).toBe('2026-11-23')
    expect(r.days).toHaveLength(17)
    expect(r.days[0]).toBe(r.from)
    expect(r.days.at(-1)).toBe(r.to)
  })
  it('handles a zero horizon and month ends', () => {
    expect(agendaRange('2026-12-01', 0).days).toEqual(['2026-11-29', '2026-11-30', '2026-12-01'])
  })
})

describe('Meal plan agenda', () => {
  it('lists days from two days ago, loads that range and marks today', async () => {
    asUser('adult')
    ready()
    const w = mount(MealPlanView, { attachTo: document.body })
    await flushPromises()
    const days = w.findAll('.agenda__day').map((d) => d.attributes('data-day'))
    expect(days[0]).toBe('2026-11-07')
    expect(days.at(-1)).toBe('2026-11-23')
    expect(w.findAll('.agenda__day.is-today')[0]!.attributes('data-day')).toBe(TODAY)
    expect(h.api.plan).toHaveBeenCalledWith('2026-11-07', '2026-11-23')
    expect(q('plan-slot-add-2026-11-08-dinner')).toBeNull()
    expect(q(`plan-slot-add-${TODAY}-dinner`)).not.toBeNull()
    w.unmount()
  })
})
