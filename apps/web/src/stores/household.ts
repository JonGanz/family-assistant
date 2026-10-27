import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api, type HouseholdPatch, type HouseholdViewExt } from '../api/client'
import { isConflict } from '../api/http'
import type { LiveEventKind } from '../composables/useLiveStream'
import { useSessionStore } from './session'

export const useHouseholdStore = defineStore('household', () => {
  const view = ref<HouseholdViewExt | null>(null)
  const error = ref<string | null>(null)
  const conflict = ref(false)
  const loaded = ref(false)

  const tz = computed(() => view.value?.tz ?? useSessionStore().household?.tz ?? 'UTC')
  const units = computed(() => view.value?.units ?? 'metric')
  const nowListSize = computed(() => view.value?.now_list_size ?? 7)
  const nowWeights = computed(() => view.value?.now_weights ?? { due: 1, flag: 0.5, weather: 0.5 })
  const quiet = computed(() => ({
    from: view.value?.quiet_from ?? '21:00',
    until: view.value?.quiet_until ?? '07:00',
  }))

  async function load(): Promise<void> {
    try {
      view.value = await api.household()
      error.value = null
      conflict.value = false
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  /** Returns true on success, false on a version conflict (banner state is set). */
  async function save(patch: HouseholdPatch): Promise<boolean> {
    try {
      view.value = await api.patchHousehold(view.value?.version ?? 0, patch)
      conflict.value = false
      return true
    } catch (e) {
      if (isConflict(e)) {
        conflict.value = true
        return false
      }
      throw e
    }
  }

  async function invalidate(kind: LiveEventKind): Promise<void> {
    if (!loaded.value) return
    if (kind === 'household.updated' || kind === 'resync') await load()
  }

  return {
    view,
    error,
    conflict,
    loaded,
    tz,
    units,
    nowListSize,
    nowWeights,
    quiet,
    load,
    save,
    invalidate,
  }
})
