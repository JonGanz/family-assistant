import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import NewRecipeSheet from '../../components/food/recipes/NewRecipeSheet.vue'
import LineMatch from '../../components/food/recipes/LineMatch.vue'
import { ApiError } from '../../api/http'
import { useRecipesStore } from '../../stores/recipes'
import { useSessionStore } from '../../stores/session'
import { resetSnackbar, useSnackbar } from '../../ui/composables/useSnackbar'
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
