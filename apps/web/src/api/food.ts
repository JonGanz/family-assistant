// M5 meals API (plan/M5.md, API table). Written by the main session's scaffold; wave-4 tasks import from it
// and report a missing or wrong call instead of editing it. `food` is the app (session cookie); `foodKiosk`
// is the same API authenticated with the kiosk's device token.
import type { components } from '../gen/api'
import { request } from './http'

type S = components['schemas']
export type RecipeSummary = S['RecipeSummary']
export type RecipePage = S['RecipePage']
export type RecipeView = S['RecipeView']
export type RecipeLineView = S['RecipeLineView']
export type RecipeInput = S['RecipeInput']
export type RatingsView = S['RatingsView']
export type MemberRating = S['MemberRating']
export type RecipeImportReq = S['RecipeImportReq']
export type RecipeDraft = S['RecipeDraft']
export type LineMatchReq = S['LineMatchReq']
export type IngredientView = S['IngredientView']
export type IngredientInput = S['IngredientInput']
export type MealsSettingsView = S['MealsSettingsView']
export type MealsSettingsPatch = S['MealsSettingsPatch']
export type MealSlotView = S['MealSlotView']
export type AisleView = S['AisleView']
export type StoreView = S['StoreView']
export type StoreAisleView = S['StoreAisleView']
export type MealPlanEntryView = S['MealPlanEntryView']
export type MealPlanEntryInput = S['MealPlanEntryInput']
export type MealPlanEntryPatch = S['MealPlanEntryPatch']
export type SwapReq = S['SwapReq']
export type PlanChangeResult = S['PlanChangeResult']
export type GroceryImpact = S['GroceryImpact']
export type MealTemplateSummary = S['MealTemplateSummary']
export type MealTemplateView = S['MealTemplateView']
export type MealTemplateInput = S['MealTemplateInput']
export type TemplateCellView = S['TemplateCellView']
export type TemplateCellInput = S['TemplateCellInput']
export type TemplateFromPlanReq = S['TemplateFromPlanReq']
export type TemplatePreview = S['TemplatePreview']
export type TemplateTarget = S['TemplateTarget']
export type TemplateApplyReq = S['TemplateApplyReq']
export type SlotRef = S['SlotRef']
export type GroceryView = S['GroceryView']
export type GroceryItemView = S['GroceryItemView']
export type StapleView = S['StapleView']
export type SourceHint = S['SourceHint']
export type GroceryAddReq = S['GroceryAddReq']
export type GroceryItemPatch = S['GroceryItemPatch']
export type GenerateReq = S['GenerateReq']
export type GenerateResult = S['GenerateResult']
export type ReconcileReq = S['ReconcileReq']
export type ReconcileResult = S['ReconcileResult']
export type DashboardMeal = S['DashboardMeal']

export interface RecipeQuery {
  q?: string
  tag?: string
  favourite?: boolean
  /** The previous page's `next_cursor`. */
  before?: string
  /** Page size, default 50, at most 200. */
  limit?: number
}

const base = '/api/v1'
const qs = (o: Record<string, string | number | boolean | undefined | null>): string => {
  const p = new URLSearchParams()
  for (const [k, v] of Object.entries(o))
    if (v !== undefined && v !== null && v !== '') p.set(k, String(v))
  const s = p.toString()
  return s ? `?${s}` : ''
}

function makeFoodApi(bearer: boolean) {
  const get = <T>(path: string) => request<T>(base + path, { bearer })
  const send = <T>(method: string, path: string, body?: unknown, version?: number) =>
    request<T>(base + path, {
      method,
      body,
      headers: version === undefined ? undefined : { 'If-Match': String(version) },
      bearer,
    })

  return {
    // Recipes
    recipes: (q: RecipeQuery = {}) => get<RecipePage>(`/recipes${qs({ ...q })}`),
    recipeTags: () => get<string[]>('/recipes/tags'),
    recipe: (id: string) => get<RecipeView>(`/recipes/${id}`),
    createRecipe: (b: RecipeInput) => send<RecipeView>('POST', '/recipes', b),
    updateRecipe: (id: string, version: number, b: RecipeInput) =>
      send<RecipeView>('PUT', `/recipes/${id}`, b, version),
    deleteRecipe: (id: string) => send<void>('DELETE', `/recipes/${id}`),
    setFavourite: (id: string, favourite: boolean) =>
      send<void>('PUT', `/recipes/${id}/favourite`, { favourite }),
    setRating: (id: string, stars: number | null) =>
      send<RatingsView>('PUT', `/recipes/${id}/rating`, { stars }),
    /** URL for an `<img src>`; the image route answers with an ETag. */
    recipeImageUrl: (id: string) => `${base}/recipes/${id}/image`,
    uploadRecipeImage: (id: string, file: Blob) =>
      request<void>(`${base}/recipes/${id}/image`, { method: 'POST', raw: file, bearer }),
    deleteRecipeImage: (id: string) => send<void>('DELETE', `/recipes/${id}/image`),
    importRecipe: (b: RecipeImportReq) => send<RecipeDraft>('POST', '/recipes/import', b),
    matchLine: (recipeId: string, lineId: string, b: LineMatchReq) =>
      send<RecipeLineView>('POST', `/recipes/${recipeId}/lines/${lineId}/match`, b),

    // Ingredient catalogue
    ingredients: (q?: string) => get<IngredientView[]>(`/ingredients${qs({ q })}`),
    createIngredient: (b: IngredientInput) => send<IngredientView>('POST', '/ingredients', b),
    updateIngredient: (id: string, b: IngredientInput) =>
      send<IngredientView>('PUT', `/ingredients/${id}`, b),
    deleteIngredient: (id: string) => send<void>('DELETE', `/ingredients/${id}`),
    mergeIngredient: (id: string, into: string) =>
      send<IngredientView>('POST', `/ingredients/${id}/merge`, { into }),

    // Settings → Meals
    settings: () => get<MealsSettingsView>('/meals/settings'),
    updateSettings: (b: MealsSettingsPatch) => send<MealsSettingsView>('PUT', '/meals/settings', b),
    slots: () => get<MealSlotView[]>('/meals/slots'),
    replaceSlots: (b: MealSlotView[]) => send<MealSlotView[]>('PUT', '/meals/slots', b),
    aisles: () => get<AisleView[]>('/meals/aisles'),
    replaceAisles: (b: AisleView[]) => send<AisleView[]>('PUT', '/meals/aisles', b),
    stores: () => get<StoreView[]>('/meals/stores'),
    replaceStores: (b: StoreView[]) => send<StoreView[]>('PUT', '/meals/stores', b),
    storeAisles: (storeId: string) => get<StoreAisleView[]>(`/meals/stores/${storeId}/aisles`),
    setStoreAisles: (storeId: string, b: StoreAisleView[]) =>
      send<StoreAisleView[]>('PUT', `/meals/stores/${storeId}/aisles`, b),
    undo: (changeId: string) => send<void>('POST', `/meals/changes/${changeId}/undo`),

    // Meal plan (dates `YYYY-MM-DD`)
    plan: (from: string, to: string) => get<MealPlanEntryView[]>(`/meal-plan${qs({ from, to })}`),
    addEntry: (b: MealPlanEntryInput) => send<PlanChangeResult>('POST', '/meal-plan', b),
    updateEntry: (id: string, version: number, b: MealPlanEntryPatch) =>
      send<PlanChangeResult>('PUT', `/meal-plan/${id}`, b, version),
    deleteEntry: (id: string) => send<PlanChangeResult>('DELETE', `/meal-plan/${id}`),
    swap: (b: SwapReq) => send<PlanChangeResult>('POST', '/meal-plan/swap', b),

    // Templates
    templates: () => get<MealTemplateSummary[]>('/meal-templates'),
    template: (id: string) => get<MealTemplateView>(`/meal-templates/${id}`),
    createTemplate: (b: MealTemplateInput) => send<MealTemplateView>('POST', '/meal-templates', b),
    updateTemplate: (id: string, version: number, b: MealTemplateInput) =>
      send<MealTemplateView>('PUT', `/meal-templates/${id}`, b, version),
    deleteTemplate: (id: string) => send<void>('DELETE', `/meal-templates/${id}`),
    templateFromPlan: (b: TemplateFromPlanReq) =>
      send<MealTemplateView>('POST', '/meal-templates/from-plan', b),
    duplicateTemplate: (id: string, name: string) =>
      send<MealTemplateView>('POST', `/meal-templates/${id}/duplicate`, { name }),
    previewTemplate: (id: string, start: string) =>
      send<TemplatePreview>('POST', `/meal-templates/${id}/preview`, { start }),
    applyTemplate: (id: string, b: TemplateApplyReq) =>
      send<PlanChangeResult>('POST', `/meal-templates/${id}/apply`, b),

    // Grocery list
    grocery: () => get<GroceryView>('/grocery'),
    addItem: (b: GroceryAddReq) => send<GroceryItemView>('POST', '/grocery/items', b),
    checkItem: (id: string) => send<GroceryItemView>('POST', `/grocery/items/${id}/check`, {}),
    uncheckItem: (id: string) => send<GroceryItemView>('POST', `/grocery/items/${id}/uncheck`, {}),
    lowItem: (id: string) => send<GroceryItemView>('POST', `/grocery/items/${id}/low`, {}),
    updateItem: (id: string, version: number, b: GroceryItemPatch) =>
      send<GroceryItemView>('PUT', `/grocery/items/${id}`, b, version),
    deleteItem: (id: string) => send<void>('DELETE', `/grocery/items/${id}`),
    generate: (b: GenerateReq) => send<GenerateResult>('POST', '/grocery/generate', b),
    reconcile: (b: ReconcileReq) => send<ReconcileResult>('POST', '/grocery/reconcile', b),
  }
}

export type FoodApi = ReturnType<typeof makeFoodApi>
export const food: FoodApi = makeFoodApi(false)
export const foodKiosk: FoodApi = makeFoodApi(true)
