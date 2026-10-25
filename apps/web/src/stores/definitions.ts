import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  api,
  type TaskDefinitionInput,
  type TaskDefinitionPatch,
  type TaskDefinitionView,
} from '../api/client'
import { isConflict } from '../api/http'

export const useDefinitionsStore = defineStore('definitions', () => {
  const items = ref<TaskDefinitionView[]>([])
  const loaded = ref(false)
  const error = ref<string | null>(null)
  const conflict = ref(false)

  async function load(): Promise<void> {
    try {
      items.value = await api.taskDefinitions()
      error.value = null
      conflict.value = false
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  async function create(b: TaskDefinitionInput): Promise<void> {
    await api.createTaskDefinition(b)
    await load()
  }

  async function patch(d: TaskDefinitionView, p: TaskDefinitionPatch): Promise<boolean> {
    try {
      await api.patchTaskDefinition(d.id, d.version, p)
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
    await api.deleteTaskDefinition(id)
    await load()
  }

  async function invalidate(kind: string): Promise<void> {
    if (!loaded.value) return
    if (kind === 'tasks.updated' || kind === 'resync') await load()
  }

  return { items, loaded, error, conflict, load, create, patch, remove, invalidate }
})
