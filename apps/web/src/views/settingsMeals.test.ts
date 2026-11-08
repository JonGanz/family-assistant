import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import TodayMeals from '../components/food/TodayMeals.vue'
import { isSlotTime, useMealsSettingsStore } from '../stores/mealsSettings'
import { resetSnackbar, useSnackbar } from '../ui/composables/useSnackbar'
import IngredientsCard from './parts/meals/IngredientsCard.vue'
import { visibleGroups } from './settingsNav'

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
  useRoute: () => ({}),
  RouterLink: { template: '<a><slot /></a>' },
}))
vi.mock('../api/food', () => ({
  food: {
    plan: vi.fn(),
    updateIngredient: vi.fn(),
    mergeIngredient: vi.fn(),
    ingredients: vi.fn(),
    settings: vi.fn(),
    slots: vi.fn(),
    aisles: vi.fn(),
    stores: vi.fn(),
  },
}))
import { food } from '../api/food'
const f = vi.mocked(food)

const ing = (id: string, name: string, staple = false) => ({
  id,
  canonical_name: name,
  aliases: [],
  is_staple: staple,
  origin: 'seed',
})

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  resetSnackbar()
})

describe('meals settings', () => {
  it('lists visible slots in order', () => {
    const s = useMealsSettingsStore()
    s.slots = [
      { name: 'Dinner', time: '18:00', visible: true, ord: 2 },
      { name: 'Lunch', time: '12:00', visible: false, ord: 1 },
      { name: 'Breakfast', time: '07:30', visible: true, ord: 0 },
    ]
    expect(s.visibleSlots.map((x) => x.name)).toEqual(['Breakfast', 'Dinner'])
  })

  it('validates slot times as HH:MM', () => {
    expect(isSlotTime('18:00')).toBe(true)
    expect(isSlotTime('07:05')).toBe(true)
    expect(isSlotTime('24:00')).toBe(false)
    expect(isSlotTime('7:30')).toBe(false)
    expect(isSlotTime('18:60')).toBe(false)
    expect(isSlotTime('')).toBe(false)
  })

  it('saves the staple switch at once and offers Undo', async () => {
    const s = useMealsSettingsStore()
    s.ingredients = [ing('i1', 'Flour')] as never
    f.updateIngredient.mockResolvedValue(ing('i1', 'Flour', true) as never)
    const w = mount(IngredientsCard)
    await w.get('[data-testid="ingredient-Flour"] [role=switch]').trigger('click')
    await flushPromises()
    expect(f.updateIngredient).toHaveBeenCalledWith(
      'i1',
      expect.objectContaining({ is_staple: true }),
    )
    expect(useSnackbar().state.current?.action?.label).toBe('Undo')
  })

  it('asks before merging', async () => {
    const s = useMealsSettingsStore()
    s.ingredients = [ing('i1', 'Flour'), ing('i2', 'Plain flour')] as never
    f.mergeIngredient.mockResolvedValue(ing('i2', 'Plain flour') as never)
    f.ingredients.mockResolvedValue([ing('i2', 'Plain flour')] as never)
    const w = mount(IngredientsCard, { attachTo: document.body })
    await w.get('[data-testid="ingredient-Flour"] button').trigger('click')
    await flushPromises()
    const sel = document.body.querySelector('select[name], select') as HTMLSelectElement
    const selects = Array.from(document.body.querySelectorAll('select'))
    const mergeSel = selects[selects.length - 1]!
    mergeSel.value = 'i2'
    mergeSel.dispatchEvent(new Event('change'))
    void sel
    await flushPromises()
    const merge = Array.from(document.body.querySelectorAll('button')).find(
      (b) => b.textContent?.trim() === 'Merge',
    )!
    merge.click()
    await flushPromises()
    expect(f.mergeIngredient).not.toHaveBeenCalled()
    const accept = Array.from(document.body.querySelectorAll('button')).filter(
      (b) => b.textContent?.trim() === 'Merge',
    )
    accept[accept.length - 1]!.click()
    await flushPromises()
    expect(f.mergeIngredient).toHaveBeenCalledWith('i1', 'i2')
    w.unmount()
  })

  it('keeps the page out of a child’s settings list', () => {
    const child = visibleGroups(false).flatMap((g) => g.pages.map((p) => p.to))
    const adult = visibleGroups(true).flatMap((g) => g.pages.map((p) => p.to))
    expect(child).not.toContain('/settings/meals')
    expect(adult).toContain('/settings/meals')
  })

  it('hides the Today card when there are no entries', async () => {
    f.plan.mockResolvedValue([])
    const s = useMealsSettingsStore()
    s.loaded = true
    s.slots = [{ id: 'sl1', name: 'Dinner', time: '18:00', visible: true, ord: 0 }] as never
    const w = mount(TodayMeals)
    await flushPromises()
    expect(w.find('[data-testid="today-meals"]').exists()).toBe(false)
  })

  it('shows today’s entries in visible slots', async () => {
    f.plan.mockResolvedValue([
      { id: 'e1', slot_id: 'sl1', ord: 0, title: 'Soup', date: '2026-11-08', version: 1 },
      { id: 'e2', slot_id: 'sl2', ord: 0, title: 'Hidden', date: '2026-11-08', version: 1 },
    ] as never)
    const s = useMealsSettingsStore()
    s.loaded = true
    s.slots = [
      { id: 'sl1', name: 'Dinner', time: '18:00', visible: true, ord: 0 },
      { id: 'sl2', name: 'Lunch', time: '12:00', visible: false, ord: 1 },
    ] as never
    const w = mount(TodayMeals, {
      global: { stubs: { RouterLink: { template: '<a><slot /></a>' } } },
    })
    await flushPromises()
    expect(w.text()).toContain('Soup')
    expect(w.text()).not.toContain('Hidden')
  })
})
