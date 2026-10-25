import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type TaskPatch, type TaskView } from '../api/client'
import { isConflict } from '../api/http'

export type TaskTab = 'now' | 'mine' | 'all' | 'suggested'

const QUERY = {
  now: { view: 'now', status: 'open' },
  mine: { view: 'mine', status: 'open' },
  all: { view: 'all', status: 'open' },
  suggested: { view: 'all', status: 'suggested' },
} as const

export interface UndoState {
  id: string
  title: string
}

export const useTasksStore = defineStore('tasks', () => {
  const tabs = ref<Partial<Record<TaskTab, TaskView[]>>>({})
  /** Calendar ranges: key `from|to` → tasks (RFC 3339 instants, `to` exclusive). */
  const ranges = ref<Record<string, TaskView[]>>({})
  const error = ref<string | null>(null)
  const conflict = ref(false)
  const undo = ref<UndoState | null>(null)
  const wanted = new Set<TaskTab>()

  const msg = (e: unknown) => (e instanceof Error ? e.message : String(e))

  async function load(tab: TaskTab): Promise<void> {
    wanted.add(tab)
    try {
      tabs.value[tab] = await api.tasks(QUERY[tab])
      error.value = null
    } catch (e) {
      error.value = msg(e)
    }
  }

  async function loadRange(from: string, to: string): Promise<void> {
    try {
      ranges.value[`${from}|${to}`] = await api.tasks({ from, to })
      error.value = null
    } catch (e) {
      error.value = msg(e)
    }
  }

  function rangeTasks(from: string, to: string): TaskView[] {
    return ranges.value[`${from}|${to}`] ?? []
  }

  async function refresh(): Promise<void> {
    await Promise.all([
      ...[...wanted].map(load),
      ...Object.keys(ranges.value).map((k) => {
        const [from, to] = k.split('|') as [string, string]
        return loadRange(from, to)
      }),
    ])
  }

  async function get(id: string): Promise<TaskView> {
    return api.task(id)
  }

  /** Completes a task and arms the 10-second undo. `member` is the acting member on a device. */
  async function complete(t: { id: string; title: string }, member?: string): Promise<void> {
    await api.completeTask(t.id, member)
    undo.value = { id: t.id, title: t.title }
    await refresh()
  }

  async function reopenDone(id: string): Promise<void> {
    await api.reopenTask(id)
    await refresh()
  }

  /** Reverts the last completion (the toast's Undo). */
  async function undoLast(): Promise<void> {
    const u = undo.value
    undo.value = null
    if (u) await reopenDone(u.id)
  }

  function expireUndo(): void {
    undo.value = null
  }

  async function act(fn: () => Promise<unknown>): Promise<void> {
    await fn()
    await refresh()
  }

  const claim = (id: string) => act(() => api.claimTask(id))
  const release = (id: string) => act(() => api.releaseTask(id))
  const accept = (id: string) => act(() => api.acceptTask(id))
  /** Dismissing a suggestion is a skip. */
  const skip = (id: string) => act(() => api.skipTask(id))
  const remove = (id: string) => act(() => api.deleteTask(id))
  const tick = (id: string, stepId: string, done: boolean) =>
    act(() => api.tickTaskStep(id, stepId, done))
  const create = (b: Parameters<typeof api.createTask>[0]) => act(() => api.createTask(b))

  /** Returns the saved task, or null on a 409 (banner state is set). */
  async function save(t: TaskView, patch: TaskPatch): Promise<TaskView | null> {
    try {
      const v = await api.patchTask(t.id, t.version, patch)
      conflict.value = false
      await refresh()
      return v
    } catch (e) {
      if (isConflict(e)) {
        conflict.value = true
        return null
      }
      throw e
    }
  }

  async function invalidate(kind: string): Promise<void> {
    if (kind === 'tasks.updated' || kind === 'resync') await refresh()
  }

  function reset(): void {
    tabs.value = {}
    ranges.value = {}
    wanted.clear()
    undo.value = null
  }

  return {
    tabs,
    ranges,
    error,
    conflict,
    undo,
    load,
    loadRange,
    rangeTasks,
    refresh,
    get,
    complete,
    reopenDone,
    undoLast,
    expireUndo,
    claim,
    release,
    accept,
    skip,
    remove,
    tick,
    create,
    save,
    invalidate,
    reset,
  }
})
