import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  api,
  type NewWeatherRule,
  type WeatherRulePatch,
  type WeatherRuleView,
} from '../api/client'
import { isConflict } from '../api/http'

export const useRulesStore = defineStore('rules', () => {
  const items = ref<WeatherRuleView[]>([])
  const error = ref<string | null>(null)
  const conflict = ref(false)

  async function load(): Promise<void> {
    try {
      items.value = await api.rules()
      error.value = null
      conflict.value = false
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  async function create(b: NewWeatherRule): Promise<void> {
    await api.createRule(b)
    await load()
  }

  async function patch(r: WeatherRuleView, p: WeatherRulePatch): Promise<boolean> {
    try {
      await api.patchRule(r.id, r.version, p)
      await load()
      return true
    } catch (e) {
      if (isConflict(e)) {
        conflict.value = true
        return false
      }
      throw e
    }
  }

  async function remove(id: string): Promise<void> {
    await api.deleteRule(id)
    await load()
  }

  return { items, error, conflict, load, create, patch, remove }
})
