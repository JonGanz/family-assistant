import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type TaskListInput, type TaskListPatch, type TaskListView } from '../api/client'
import { isConflict } from '../api/http'

export const useListsStore = defineStore('lists', () => {
  const items = ref<TaskListView[]>([])
  const loaded = ref(false)
  const error = ref<string | null>(null)
  const conflict = ref(false)

  async function load(): Promise<void> {
    try {
      items.value = (await api.taskLists()).sort((a, b) => a.sort - b.sort)
      error.value = null
      conflict.value = false
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  async function create(b: TaskListInput): Promise<void> {
    await api.createTaskList(b)
    await load()
  }

  /** Returns false on a 409 (banner state is set). */
  async function patch(l: TaskListView, p: TaskListPatch): Promise<boolean> {
    try {
      await api.patchTaskList(l.id, l.version, p)
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
    await api.deleteTaskList(id)
    await load()
  }

  async function invalidate(kind: string): Promise<void> {
    if (!loaded.value) return
    if (kind === 'tasks.updated' || kind === 'resync') await load()
  }

  return { items, loaded, error, conflict, load, create, patch, remove, invalidate }
})
