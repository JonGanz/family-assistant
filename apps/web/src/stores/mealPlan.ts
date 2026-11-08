import { defineStore } from 'pinia'
import { computed, ref, shallowRef } from 'vue'
import {
  food,
  type FoodApi,
  type GenerateReq,
  type GenerateResult,
  type GroceryImpact,
  type MealPlanEntryInput,
  type MealPlanEntryPatch,
  type MealPlanEntryView,
  type MealTemplateInput,
  type MealTemplateSummary,
  type MealTemplateView,
  type PlanChangeResult,
  type ReconcileResult,
  type SwapReq,
  type TemplateApplyReq,
  type TemplateFromPlanReq,
  type TemplatePreview,
} from '../api/food'
import { addDays } from '../lib/time'

/** `GET /meal-plan` takes at most 62 days; longer ranges are fetched in chunks of eight weeks. */
const CHUNK_DAYS = 56

export const cellKey = (date: string, slotId: string) => `${date}|${slotId}`

/** The plan in a date range, the templates, and the writes with their grocery impact (D-116, D-120). */
export const useMealPlanStore = defineStore('mealPlan', () => {
  /** The kiosk's grid swaps this for the device-token API (`useApi`). */
  const api = shallowRef<FoodApi>(food)
  const entries = ref<MealPlanEntryView[]>([])
  const range = ref<{ from: string; to: string } | null>(null)
  const loaded = ref(false)
  const error = ref<string | null>(null)
  const templates = ref<MealTemplateSummary[]>([])
  const templatesLoaded = ref(false)
  /** The list impact of the last plan write (D-116); empty means nothing to ask. */
  const impact = ref<GroceryImpact[]>([])
  let seq = 0

  /** Entries of every (date, slot) cell in `ord` order. */
  const byCell = computed(() => {
    const m = new Map<string, MealPlanEntryView[]>()
    for (const e of entries.value) {
      const k = cellKey(e.date, e.slot_id)
      const l = m.get(k)
      if (l) l.push(e)
      else m.set(k, [e])
    }
    for (const l of m.values()) l.sort((a, b) => a.ord - b.ord)
    return m
  })
  const cell = (date: string, slotId: string): MealPlanEntryView[] =>
    byCell.value.get(cellKey(date, slotId)) ?? []

  function useApi(a: FoodApi) {
    api.value = a
  }

  /** Loads `from..to` inclusive (`YYYY-MM-DD`). A newer call wins over an older one still in flight. */
  async function load(from: string, to: string): Promise<void> {
    range.value = { from, to }
    const mine = ++seq
    try {
      const parts: Promise<MealPlanEntryView[]>[] = []
      for (let s = from; s <= to; s = addDays(s, CHUNK_DAYS)) {
        const end = addDays(s, CHUNK_DAYS - 1)
        parts.push(api.value.plan(s, end < to ? end : to))
      }
      const all = (await Promise.all(parts)).flat()
      if (mine !== seq) return
      entries.value = all
      error.value = null
    } catch (e) {
      if (mine === seq) error.value = e instanceof Error ? e.message : String(e)
    } finally {
      if (mine === seq) loaded.value = true
    }
  }

  async function reload(): Promise<void> {
    if (range.value) await load(range.value.from, range.value.to)
  }

  async function loadTemplates(): Promise<void> {
    try {
      templates.value = await api.value.templates()
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      templatesLoaded.value = true
    }
  }

  /** Every plan write comes back through here: refetch, then expose the list impact. */
  async function written(res: PlanChangeResult): Promise<PlanChangeResult> {
    impact.value = res.grocery_impact
    await reload()
    return res
  }

  const add = async (b: MealPlanEntryInput) => written(await api.value.addEntry(b))
  const update = async (e: MealPlanEntryView, b: MealPlanEntryPatch) =>
    written(await api.value.updateEntry(e.id, e.version, b))
  /** The server returns no change for a delete; the caller restores with `add` of the same content. */
  const remove = async (e: MealPlanEntryView) => written(await api.value.deleteEntry(e.id))
  const swap = async (b: SwapReq) => written(await api.value.swap(b))
  const preview = (id: string, start: string): Promise<TemplatePreview> =>
    api.value.previewTemplate(id, start)
  const apply = async (id: string, b: TemplateApplyReq) =>
    written(await api.value.applyTemplate(id, b))

  async function undo(changeId: string): Promise<void> {
    await api.value.undo(changeId)
    await reload()
  }

  /** "Update the list" (D-116): recompute the contributions of the entries in the impact. */
  async function reconcile(): Promise<ReconcileResult | null> {
    const ids = [...new Set(impact.value.map((i) => i.entry_id))]
    impact.value = []
    return ids.length ? api.value.reconcile({ entry_ids: ids }) : null
  }
  /** Make grocery list from picked entries or a date range. */
  const generate = (b: GenerateReq): Promise<GenerateResult> => api.value.generate(b)
  /** "Leave the list". */
  function dismissImpact() {
    impact.value = []
  }

  const template = (id: string): Promise<MealTemplateView> => api.value.template(id)
  async function saveTemplate(
    id: string | null,
    version: number,
    b: MealTemplateInput,
  ): Promise<MealTemplateView> {
    const v = id
      ? await api.value.updateTemplate(id, version, b)
      : await api.value.createTemplate(b)
    await loadTemplates()
    return v
  }
  async function deleteTemplate(id: string): Promise<void> {
    await api.value.deleteTemplate(id)
    await loadTemplates()
  }
  async function duplicateTemplate(id: string, name: string): Promise<MealTemplateView> {
    const v = await api.value.duplicateTemplate(id, name)
    await loadTemplates()
    return v
  }
  async function fromPlan(b: TemplateFromPlanReq): Promise<MealTemplateView> {
    const v = await api.value.templateFromPlan(b)
    await loadTemplates()
    return v
  }

  async function invalidate(kind: string): Promise<void> {
    if (kind !== 'mealplan.updated' && kind !== 'resync') return
    await Promise.all([
      loaded.value ? reload() : null,
      templatesLoaded.value ? loadTemplates() : null,
    ])
  }

  return {
    entries,
    range,
    loaded,
    error,
    templates,
    templatesLoaded,
    impact,
    cell,
    useApi,
    load,
    loadTemplates,
    add,
    update,
    remove,
    swap,
    preview,
    apply,
    undo,
    reconcile,
    generate,
    dismissImpact,
    template,
    saveTemplate,
    deleteTemplate,
    duplicateTemplate,
    fromPlan,
    invalidate,
  }
})
