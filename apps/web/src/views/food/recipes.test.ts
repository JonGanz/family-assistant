import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import NewRecipeSheet from '../../components/food/recipes/NewRecipeSheet.vue'
import LineMatch from '../../components/food/recipes/LineMatch.vue'
import { ApiError } from '../../api/http'
import { useRecipesStore } from '../../stores/recipes'
import { useSessionStore } from '../../stores/session'
import { resetSnackbar, useSnackbar } from '../../ui/composables/useSnackbar'
import {
  deleteSection,
  fromDraft,
  fromView,
  mergeSection,
  moveItem,
  toInput,
} from '../../components/food/recipes/sectionsForm'
import { applyReimport } from '../../stores/recipesReimport'
import RecipeEditView from './RecipeEditView.vue'
import RecipeView from './RecipeView.vue'
import RecipesView from './RecipesView.vue'

const push = vi.fn()
const routeState = { params: { id: 'r1' } as Record<string, string> }
vi.mock('vue-router', async (orig) => ({
  ...(await orig<typeof import('vue-router')>()),
  useRouter: () => ({ push, replace: vi.fn(), go: vi.fn() }),
  useRoute: () => ({ params: routeState.params, query: {} }),
  onBeforeRouteLeave: vi.fn(),
}))
vi.mock('../../api/food', () => ({
  food: {
    recipes: vi.fn(),
    recipeTags: vi.fn(),
    recipe: vi.fn(),
    createRecipe: vi.fn(),
    updateRecipe: vi.fn(),
    setRating: vi.fn(),
    importRecipe: vi.fn(),
    matchLine: vi.fn(),
    ingredients: vi.fn(),
    recipeImageUrl: (id: string) => `/img/${id}`,
    uploadRecipeImage: vi.fn(),
  },
}))
import { food } from '../../api/food'
const m = vi.mocked(food)

const summary = (id: string, tags: string[]) => ({
  id,
  title: `Recipe ${id}`,
  has_image: false,
  tags,
  favourite: false,
})
const view = (over = {}) =>
  ({
    id: 'r1',
    title: 'Soup',
    import_method: 'manual',
    tags: [],
    favourite: false,
    has_image: false,
    lines: [],
    steps: ['Boil'],
    ratings: { by_member: [], mine: null, avg: null },
    version: 1,
    ...over,
  }) as never

function asRole(role: 'adult' | 'child') {
  const s = useSessionStore()
  ;(s as unknown as { me: unknown }).me = { actor: { kind: 'user', member: { id: 'a', role } } }
}
const stubs = { RouterLink: { template: '<a><slot /></a>' }, FoodTabs: true }

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  resetSnackbar()
  push.mockReset()
  routeState.params = { id: 'r1' }
})

describe('recipes', () => {
  it('shows the import error fallback', async () => {
    const store = useRecipesStore()
    m.importRecipe.mockRejectedValue(new ApiError(422, { code: 'no_recipe' }))
    const w = mount(NewRecipeSheet, {
      props: { modelValue: true },
      global: { stubs: { teleport: true } },
    })
    await w.find('[data-testid="recipe-import-url"]').trigger('click')
    await w.find('input').setValue('https://example.com/r')
    await w.find('[data-testid="recipe-import-submit"]').trigger('click')
    await flushPromises()
    expect(w.find('[data-testid="recipe-import-error"]').text()).toContain('no recipe')
    expect(w.find('[data-testid="recipe-type-in"]').exists()).toBe(true)
    expect(store.pendingDraft).toBeNull()
  })

  it('does not store an imported draft before Save', async () => {
    const store = useRecipesStore()
    store.pendingDraft = {
      name: 'Pasta',
      origin: 'jsonld',
      ingredients: [{ text: '200 g pasta' }],
      steps: ['Cook'],
      tags: [],
    } as never
    routeState.params = {}
    const w = mount(RecipeEditView, { global: { stubs } })
    await flushPromises()
    expect((w.find('[data-testid="recipe-title"]').element as HTMLInputElement).value).toBe('Pasta')
    expect(m.createRecipe).not.toHaveBeenCalled()
    expect(m.updateRecipe).not.toHaveBeenCalled()
    w.unmount()
  })

  it('shows a visible error when the image cannot be decoded', async () => {
    routeState.params = {}
    const w = mount(RecipeEditView, { global: { stubs } })
    await flushPromises()
    const input = w.find('input[type="file"]')
    expect(input.attributes('accept')).toBe('image/*')
    Object.defineProperty(input.element, 'files', {
      value: [new File(['x'], 'a.heic', { type: 'image/heic' })],
    })
    await input.trigger('change')
    await flushPromises()
    expect(w.find('[data-testid="image-error"]').text()).toContain('could not be read')
    w.unmount()
  })

  it('Accept on a proposal calls the match route', async () => {
    m.matchLine.mockResolvedValue({ id: 'l1' } as never)
    const w = mount(LineMatch, {
      props: {
        recipeId: 'r1',
        line: {
          id: 'l1',
          ord: 0,
          raw_text: 'a pinch of saffron',
          match_state: 'proposed',
          proposal: { confidence: 0.8, ingredient_id: 'i9', new_name: null },
        } as never,
      },
    })
    await w.find('[data-testid="recipe-proposal-accept"]').trigger('click')
    await flushPromises()
    expect(m.matchLine).toHaveBeenCalledWith('r1', 'l1', { ingredient_id: 'i9' })
  })

  it('rating saves instantly and offers Undo', async () => {
    asRole('adult')
    m.recipe.mockResolvedValue(view())
    m.setRating.mockResolvedValue({ by_member: [], mine: 4, avg: 4 } as never)
    const w = mount(RecipeView, { global: { stubs } })
    await flushPromises()
    await w.find('[data-testid="recipe-rating-4"]').trigger('click')
    await flushPromises()
    expect(m.setRating).toHaveBeenCalledWith('r1', 4)
    const snack = useSnackbar()
    expect(snack.state.current?.action?.label).toBe('Undo')
    await snack.state.current?.action?.run()
    expect(m.setRating).toHaveBeenLastCalledWith('r1', null)
  })

  it('hides New, Edit and Delete from a child', async () => {
    asRole('child')
    m.recipe.mockResolvedValue(view())
    m.recipes.mockResolvedValue({ items: [summary('a', [])] })
    m.recipeTags.mockResolvedValue([])
    const v = mount(RecipeView, { global: { stubs } })
    const l = mount(RecipesView, { global: { stubs } })
    await flushPromises()
    expect(v.find('[data-testid="recipe-edit"]').exists()).toBe(false)
    expect(v.find('[data-testid="recipe-delete"]').exists()).toBe(false)
    expect(l.find('[data-testid="recipe-new"]').exists()).toBe(false)
  })

  it('tag chips filter the list', async () => {
    asRole('adult')
    m.recipes.mockResolvedValue({ items: [summary('a', ['quick']), summary('b', ['slow'])] })
    m.recipeTags.mockResolvedValue(['quick', 'slow'])
    const w = mount(RecipesView, { global: { stubs } })
    await flushPromises()
    expect(w.findAll('[data-testid^="recipe-card-"]')).toHaveLength(2)
    const chip = w.findAll('button.ui-fchip').find((b) => b.text() === 'quick')!
    await chip.trigger('click')
    expect(w.findAll('[data-testid^="recipe-card-"]')).toHaveLength(1)
  })
})

describe('recipe sections', () => {
  const sectioned = () =>
    view({
      lines: [
        { id: 'l0', ord: 0, raw_text: '1 lime', match_state: 'matched', section_id: null },
        { id: 'l1', ord: 1, raw_text: '8 drumsticks', match_state: 'matched', section_id: 's1' },
        { id: 'l2', ord: 2, raw_text: '3 tbsp honey', match_state: 'matched', section_id: 's2' },
      ],
      steps: [
        { id: 't0', ord: 0, text: 'Roast', section_id: 's1' },
        { id: 't1', ord: 1, text: 'Warm', section_id: 's2' },
        { id: 't2', ord: 2, text: 'Serve', section_id: null },
      ],
      sections: [
        { id: 's1', ord: 0, name: 'Chicken' },
        { id: 's2', ord: 1, name: 'Glaze' },
      ],
    })

  it('the page renders shared, two sections and finish in that order', async () => {
    asRole('adult')
    m.recipe.mockResolvedValue(sectioned())
    const w = mount(RecipeView, { global: { stubs } })
    await flushPromises()
    const html = w.html()
    const at = (t: string) => html.indexOf(t)
    expect(at('1 lime')).toBeGreaterThan(-1)
    expect(at('1 lime')).toBeLessThan(at('Chicken'))
    expect(at('Chicken')).toBeLessThan(at('Glaze'))
    expect(at('Glaze')).toBeLessThan(at('Serve'))
    expect(w.findAll('[data-testid^="recipe-section-"]')).toHaveLength(2)
    expect(w.find('[data-testid="recipe-finish"]').text()).toContain('Finish')
  })

  it('a recipe without sections keeps the Steps heading', async () => {
    asRole('adult')
    m.recipe.mockResolvedValue(
      view({ steps: [{ id: 't', ord: 0, text: 'Boil', section_id: null }], sections: [] }),
    )
    const w = mount(RecipeView, { global: { stubs } })
    await flushPromises()
    expect(w.find('[data-testid="recipe-finish"]').text()).toContain('Steps')
    expect(w.find('[data-testid="recipe-sections"]').exists()).toBe(false)
  })

  it('the editor round-trips sections into RecipeInput', async () => {
    asRole('adult')
    m.recipe.mockResolvedValue(sectioned())
    m.updateRecipe.mockResolvedValue(sectioned())
    const w = mount(RecipeEditView, { global: { stubs } })
    await flushPromises()
    expect(w.find('[data-testid="recipe-section-edit-1"]').exists()).toBe(true)
    await w.find('[data-testid="recipe-title"]').setValue('Drumsticks')
    await w
      .findAll('[data-testid="recipe-save"] button')
      .find((b) => b.text() === 'Save')!
      .trigger('click')
    await flushPromises()
    expect(m.updateRecipe).toHaveBeenCalled()
    const body = m.updateRecipe.mock.calls[0]![2]
    expect(body.lines).toEqual(['1 lime'])
    expect(body.steps).toEqual(['Serve'])
    expect(body.sections).toEqual([
      { name: 'Chicken', lines: ['8 drumsticks'], steps: ['Roast'] },
      { name: 'Glaze', lines: ['3 tbsp honey'], steps: ['Warm'] },
    ])
  })

  it('merge appends to the target; delete moves items to shared and finish', () => {
    const f = fromView(sectioned() as never)
    mergeSection(f, 1, 0)
    expect(f.sections).toEqual([
      { name: 'Chicken', lines: ['8 drumsticks', '3 tbsp honey'], steps: ['Roast', 'Warm'] },
    ])
    deleteSection(f, 0)
    expect(f.sections).toEqual([])
    expect(f.lines).toEqual(['1 lime', '8 drumsticks', '3 tbsp honey'])
    expect(f.steps).toEqual(['Serve', 'Roast', 'Warm'])
  })

  it('moves a line and a step between places and builds a form from a draft', () => {
    const f = fromDraft({
      ingredients: [{ text: 'a' }],
      steps: ['x'],
      sections: [{ name: 'S', ingredients: [{ text: 'b' }], steps: ['y'] }],
    } as never)
    moveItem(f, 'lines', 'top', 0, 0)
    moveItem(f, 'steps', 0, 0, 'top')
    expect(f.sections[0]).toEqual({ name: 'S', lines: ['b', 'a'], steps: [] })
    expect(f.steps).toEqual(['x', 'y'])
    expect(toInput(f).sections).toEqual([{ name: 'S', lines: ['b', 'a'], steps: [] }])
  })

  it('re-import replaces ingredients, steps, sections and times and keeps title, tags, note', () => {
    const form = {
      title: 'Mine',
      tags: 'quick',
      note: 'less salt',
      servings: '4',
      prep: '5',
      cook: '',
      total: '',
      lines: ['old line'],
      steps: ['old step'],
      sections: [],
    }
    const meta = applyReimport(
      form as never,
      {
        name: 'Other title',
        origin: 'jsonld',
        yield_text: '6 servings',
        servings: 6,
        prep_minutes: 10,
        cook_minutes: 20,
        total_minutes: 30,
        tags: ['x'],
        ingredients: [{ text: '1 egg' }],
        steps: ['Serve'],
        sections: [{ name: 'Base', ingredients: [{ text: '2 eggs' }], steps: ['Whisk'] }],
      } as never,
    )
    expect(form.title).toBe('Mine')
    expect(form.tags).toBe('quick')
    expect(form.note).toBe('less salt')
    expect(form.lines).toEqual(['1 egg'])
    expect(form.steps).toEqual(['Serve'])
    expect(form.sections).toEqual([{ name: 'Base', lines: ['2 eggs'], steps: ['Whisk'] }])
    expect([form.servings, form.prep, form.cook, form.total]).toEqual(['6', '10', '20', '30'])
    expect(meta).toEqual({ import_method: 'jsonld', yield_text: '6 servings' })
  })

  it('the editor re-imports from the source URL and leaves the form on failure', async () => {
    m.recipe.mockResolvedValue(view({ source_url: 'https://example.com/r', lines: [] }))
    asRole('adult')
    const w = mount(RecipeEditView, { global: { stubs: { ...stubs, teleport: true } } })
    await flushPromises()
    m.importRecipe.mockRejectedValueOnce(new Error('boom'))
    await w.find('[data-testid="recipe-reimport"]').trigger('click')
    await w.find('[data-testid="recipe-reimport-confirm"]').trigger('click')
    await flushPromises()
    expect(m.importRecipe).toHaveBeenCalledWith({ url: 'https://example.com/r' })
    expect(w.find('[data-testid="recipe-reimport-error"]').text()).toContain('boom')
    m.importRecipe.mockResolvedValueOnce({
      name: 'X',
      origin: 'jsonld',
      ingredients: [{ text: '1 egg' }],
      steps: ['Serve'],
      tags: [],
    } as never)
    await w.find('[data-testid="recipe-reimport"]').trigger('click')
    await w.find('[data-testid="recipe-reimport-confirm"]').trigger('click')
    await flushPromises()
    expect(w.find('[data-testid="recipe-reimport-notice"]').exists()).toBe(true)
    expect(m.updateRecipe).not.toHaveBeenCalled()
    w.unmount()
  })
})
