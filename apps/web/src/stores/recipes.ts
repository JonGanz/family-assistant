import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  food,
  type LineMatchReq,
  type RatingsView,
  type RecipeDraft,
  type RecipeInput,
  type RecipeLineView,
  type RecipeSummary,
  type RecipeView,
} from '../api/food'
import { isConflict } from '../api/http'

export const useRecipesStore = defineStore('recipes', () => {
  const items = ref<RecipeSummary[]>([])
  const nextCursor = ref<string | null>(null)
  const loaded = ref(false)
  const error = ref<string | null>(null)
  const query = ref<{ q: string; tags: string[]; favourite: boolean }>({
    q: '',
    tags: [],
    favourite: false,
  })
  const current = ref<RecipeView | null>(null)
  /** An imported draft, held in memory until the editor's Save (D-109). */
  const pendingDraft = ref<RecipeDraft | null>(null)
  const conflict = ref(false)

  function params(before?: string) {
    const q = query.value
    return {
      q: q.q.trim() || undefined,
      // The API filters on one tag; extra chips narrow the loaded page client-side.
      tag: q.tags[0],
      favourite: q.favourite || undefined,
      before,
    }
  }

  async function load(): Promise<void> {
    try {
      const p = await food.recipes(params())
      items.value = p.items
      nextCursor.value = p.next_cursor ?? null
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  async function more(): Promise<void> {
    if (!nextCursor.value) return
    const p = await food.recipes(params(nextCursor.value))
    items.value = [...items.value, ...p.items]
    nextCursor.value = p.next_cursor ?? null
  }

  async function open(id: string): Promise<void> {
    try {
      current.value = await food.recipe(id)
      error.value = null
    } catch (e) {
      current.value = null
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  /** Create (no `id`) or update. Returns the saved recipe, or null on a 409 (`conflict` is set). */
  async function save(id: string | null, b: RecipeInput): Promise<RecipeView | null> {
    try {
      const v =
        id && current.value?.id === id
          ? await food.updateRecipe(id, current.value.version, b)
          : await food.createRecipe(b)
      current.value = v
      conflict.value = false
      return v
    } catch (e) {
      if (isConflict(e)) {
        conflict.value = true
        return null
      }
      throw e
    }
  }

  async function remove(id: string): Promise<void> {
    await food.deleteRecipe(id)
    items.value = items.value.filter((r) => r.id !== id)
    if (current.value?.id === id) current.value = null
  }

  async function favourite(id: string, on: boolean): Promise<void> {
    await food.setFavourite(id, on)
    if (current.value?.id === id) current.value.favourite = on
    const s = items.value.find((r) => r.id === id)
    if (s) s.favourite = on
  }

  async function rate(id: string, stars: number | null): Promise<RatingsView> {
    const r = await food.setRating(id, stars)
    if (current.value?.id === id) current.value.ratings = r
    const s = items.value.find((x) => x.id === id)
    if (s) {
      s.my_rating = r.mine ?? null
      s.rating_avg = r.avg ?? null
    }
    return r
  }

  async function matchLine(
    recipeId: string,
    lineId: string,
    b: LineMatchReq,
  ): Promise<RecipeLineView> {
    const l = await food.matchLine(recipeId, lineId, b)
    if (current.value?.id === recipeId)
      current.value.lines = current.value.lines.map((x) => (x.id === l.id ? l : x))
    return l
  }

  async function importDraft(req: { url?: string; text?: string }): Promise<RecipeDraft> {
    const d = await food.importRecipe(req)
    pendingDraft.value = d
    return d
  }

  async function invalidate(kind: string): Promise<void> {
    if (kind !== 'recipes.updated' && kind !== 'resync') return
    if (loaded.value) await load()
    if (current.value) {
      const id = current.value.id
      try {
        current.value = await food.recipe(id)
      } catch {
        /* deleted elsewhere; the view shows its error state on next open */
      }
    }
  }

  return {
    items,
    nextCursor,
    loaded,
    error,
    query,
    current,
    pendingDraft,
    conflict,
    load,
    more,
    open,
    save,
    remove,
    favourite,
    rate,
    matchLine,
    importDraft,
    invalidate,
  }
})
