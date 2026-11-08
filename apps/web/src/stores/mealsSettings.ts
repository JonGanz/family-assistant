import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  food,
  type AisleView,
  type IngredientInput,
  type IngredientView,
  type MealSlotView,
  type MealsSettingsPatch,
  type MealsSettingsView,
  type StoreAisleView,
  type StoreView,
} from '../api/food'

export type SlotLike = MealSlotView

/** `"HH:MM"` (24 h), the wire format of a slot's time. */
export const isSlotTime = (s: string): boolean => /^([01]\d|2[0-3]):[0-5]\d$/.test(s)

/** Shared by Settings → Meals and the Food screens; every member may read, writes are adult only. */
export const useMealsSettingsStore = defineStore('mealsSettings', () => {
  const settings = ref<MealsSettingsView | null>(null)
  const slots = ref<MealSlotView[]>([])
  const aisles = ref<AisleView[]>([])
  const stores = ref<StoreView[]>([])
  const ingredients = ref<IngredientView[]>([])
  const loaded = ref(false)
  const error = ref<string | null>(null)

  const visibleSlots = computed(() =>
    slots.value.filter((s) => s.visible).sort((a, b) => a.ord - b.ord),
  )

  async function load(): Promise<void> {
    try {
      const [se, sl, ai, st, ing] = await Promise.all([
        food.settings(),
        food.slots(),
        food.aisles(),
        food.stores(),
        food.ingredients(),
      ])
      settings.value = se
      slots.value = sl
      aisles.value = ai
      stores.value = st
      ingredients.value = ing
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  async function saveSettings(patch: MealsSettingsPatch): Promise<void> {
    settings.value = await food.updateSettings(patch)
  }
  async function saveSlots(list: MealSlotView[]): Promise<void> {
    slots.value = await food.replaceSlots(list.map((s, i) => ({ ...s, ord: i })))
  }
  async function saveAisles(list: AisleView[]): Promise<void> {
    aisles.value = await food.replaceAisles(list.map((a, i) => ({ ...a, ord: i })))
  }
  async function saveStores(list: StoreView[]): Promise<void> {
    stores.value = await food.replaceStores(list.map((s, i) => ({ ...s, ord: i })))
  }
  const storeAisles = (id: string): Promise<StoreAisleView[]> => food.storeAisles(id)
  const saveStoreAisles = (id: string, list: StoreAisleView[]): Promise<StoreAisleView[]> =>
    food.setStoreAisles(
      id,
      list.map((a, i) => ({ ...a, ord: i })),
    )

  async function createIngredient(b: IngredientInput): Promise<IngredientView> {
    const v = await food.createIngredient(b)
    ingredients.value = [...ingredients.value, v]
    return v
  }
  async function updateIngredient(id: string, b: IngredientInput): Promise<IngredientView> {
    const v = await food.updateIngredient(id, b)
    ingredients.value = ingredients.value.map((i) => (i.id === id ? v : i))
    return v
  }
  async function deleteIngredient(id: string): Promise<void> {
    await food.deleteIngredient(id)
    ingredients.value = ingredients.value.filter((i) => i.id !== id)
  }
  /** Merges `id` into `into`; the catalogue is reloaded because aliases and recipe lines moved. */
  async function merge(id: string, into: string): Promise<void> {
    await food.mergeIngredient(id, into)
    ingredients.value = await food.ingredients()
  }

  async function invalidate(kind: string): Promise<void> {
    if (kind === 'household.updated' || kind === 'resync') await load()
  }

  return {
    settings,
    slots,
    visibleSlots,
    aisles,
    stores,
    ingredients,
    loaded,
    error,
    load,
    saveSettings,
    saveSlots,
    saveAisles,
    saveStores,
    storeAisles,
    saveStoreAisles,
    createIngredient,
    updateIngredient,
    deleteIngredient,
    merge,
    invalidate,
  }
})
