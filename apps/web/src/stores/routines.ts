import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  api,
  type RoutineInput,
  type RoutinePatch,
  type RoutineRunView,
  type RoutineView,
} from '../api/client'
import { isConflict } from '../api/http'

export const useRoutinesStore = defineStore('routines', () => {
  const items = ref<RoutineView[]>([])
  const today = ref<RoutineRunView[]>([])
  const loaded = ref(false)
  const todayLoaded = ref(false)
  const error = ref<string | null>(null)
  const conflict = ref(false)

  async function load(): Promise<void> {
    try {
      items.value = await api.routines()
      error.value = null
      conflict.value = false
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  async function loadToday(member?: string): Promise<void> {
    try {
      today.value = await api.routinesToday(member)
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      todayLoaded.value = true
    }
  }

  async function create(b: RoutineInput): Promise<void> {
    await api.createRoutine(b)
    await load()
  }

  async function patch(r: RoutineView, p: RoutinePatch): Promise<boolean> {
    try {
      await api.patchRoutine(r.id, r.version, p)
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
    await api.deleteRoutine(id)
    await load()
  }

  async function tick(run: RoutineRunView, stepId: string, done: boolean): Promise<void> {
    const v = await api.tickRoutineStep(run.id, stepId, done)
    const i = today.value.findIndex((r) => r.id === run.id)
    if (i >= 0) today.value[i] = v
  }

  async function invalidate(kind: string): Promise<void> {
    if (kind === 'routines.updated' || kind === 'resync') {
      await Promise.all([loaded.value ? load() : null, todayLoaded.value ? loadToday() : null])
    }
  }

  return {
    items,
    today,
    loaded,
    todayLoaded,
    error,
    conflict,
    load,
    loadToday,
    create,
    patch,
    remove,
    tick,
    invalidate,
  }
})
