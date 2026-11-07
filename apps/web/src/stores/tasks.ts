import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api, type TaskPatch, type TaskView } from '../api/client'
import { isConflict } from '../api/http'
import { useSessionStore } from './session'

export type TaskTab = 'now' | 'mine' | 'all' | 'suggested'

const QUERY = {
  now: { view: 'now', status: 'open' },
  mine: { view: 'mine', status: 'open' },
  all: { view: 'all', status: 'open' },
  suggested: { view: 'all', status: 'suggested' },
} as const

/** Who the task lists show (D-105): member ids; empty means everyone. */
export type TaskFilter = string[]
export const FILTER_KEY = 'fa.tasks.filter'

/**
 * Reads the stored filter. Old values were a string: 'all' is `[]`, 'mine' and a member id are
 * `[id]`. Ids that are not members are dropped by `validateFilter`.
 */
export function parseFilter(raw: string | null, fallback: TaskFilter, myId?: string): TaskFilter {
  if (raw === null || raw === '') return fallback
  if (raw === 'all') return []
  if (raw === 'mine') return myId ? [myId] : fallback
  try {
    const v: unknown = JSON.parse(raw)
    if (Array.isArray(v)) return v.filter((x): x is string => typeof x === 'string')
  } catch {
    // Not JSON: an old plain member id.
  }
  return [raw]
}
function readFilter(fallback: TaskFilter, myId?: string): TaskFilter {
  try {
    return parseFilter(localStorage.getItem(FILTER_KEY), fallback, myId)
  } catch {
    return fallback
  }
}
function writeFilter(f: TaskFilter): void {
  try {
    localStorage.setItem(FILTER_KEY, JSON.stringify(f))
  } catch {
    // Private mode or blocked storage: the choice simply is not remembered.
  }
}

/** The server pages newest-created first; the lists show the soonest due first (undated last). */
function byDue(a: TaskView, b: TaskView): number {
  if (a.due_at && b.due_at) return a.due_at < b.due_at ? -1 : a.due_at > b.due_at ? 1 : 0
  if (a.due_at) return -1
  if (b.due_at) return 1
  return 0
}

export interface UndoState {
  id: string
  title: string
}

export const useTasksStore = defineStore('tasks', () => {
  const tabs = ref<Partial<Record<TaskTab, TaskView[]>>>({})
  /** Cursor of the next page per tab; absent when the tab is fully loaded. */
  const nextBefore = ref<Partial<Record<TaskTab, string>>>({})
  /** Calendar ranges: key `from|to` → tasks (RFC 3339 instants, `to` exclusive). */
  const ranges = ref<Record<string, TaskView[]>>({})
  const error = ref<string | null>(null)
  const pending = ref(0)
  const loading = computed(() => pending.value > 0)
  const conflict = ref(false)
  const undo = ref<UndoState | null>(null)
  const wanted = new Set<TaskTab>()
  const session = useSessionStore()
  /** Per device: default is the viewer's own member (Mine), or everyone with no signed-in member. */
  const mine = (): TaskFilter => (session.member ? [session.member.id] : [])
  const filter = ref<TaskFilter>(readFilter(mine(), session.member?.id))

  /** The API takes one member: several are fetched one by one and merged. */
  function query(tab: TaskTab, member?: string) {
    return member ? { ...QUERY[tab], member } : { ...QUERY[tab] }
  }
  const memberIds = (tab: TaskTab): (string | undefined)[] =>
    tab === 'suggested' || filter.value.length === 0 ? [undefined] : filter.value

  /** Picks a filter, remembers it, drops the pages loaded so far (paging restarts) and reloads. */
  async function setFilter(f: TaskFilter): Promise<void> {
    filter.value = [...f]
    writeFilter(filter.value)
    tabs.value = {}
    nextBefore.value = {}
    await refresh()
  }
  /** Ids that are no longer members are dropped; none left falls back to Mine. */
  function validateFilter(ids: string[]): void {
    const kept = filter.value.filter((id) => ids.includes(id))
    if (kept.length === filter.value.length) return
    filter.value = kept.length ? kept : mine()
    writeFilter(filter.value)
  }

  const msg = (e: unknown) => (e instanceof Error ? e.message : String(e))

  function setTab(tab: TaskTab, items: TaskView[], next: string | null | undefined): void {
    tabs.value[tab] = tab === 'now' ? items : [...items].sort(byDue)
    if (next) nextBefore.value[tab] = next
    else delete nextBefore.value[tab]
  }

  /** Every page for each chosen member, merged by id (several members have no single cursor). */
  async function loadMerged(tab: TaskTab, ids: string[]): Promise<TaskView[]> {
    const byId = new Map<string, TaskView>()
    for (const id of ids) {
      let page = await api.tasks(query(tab, id))
      for (const t of page.items) byId.set(t.id, t)
      for (let n = 0; page.next_before && n < 20; n++) {
        page = await api.tasks({ ...query(tab, id), before: page.next_before })
        for (const t of page.items) byId.set(t.id, t)
      }
    }
    return [...byId.values()]
  }

  /** Reloads the tab's first page, and further pages until as many rows as were shown are back. */
  async function load(tab: TaskTab): Promise<void> {
    wanted.add(tab)
    pending.value++
    try {
      const ids = memberIds(tab)
      if (ids.length > 1) {
        setTab(tab, await loadMerged(tab, ids as string[]), null)
      } else {
        const shown = tabs.value[tab]?.length ?? 0
        let page = await api.tasks(query(tab, ids[0]))
        const items = [...page.items]
        while (page.next_before && items.length < shown) {
          page = await api.tasks({ ...query(tab, ids[0]), before: page.next_before })
          items.push(...page.items)
        }
        setTab(tab, items, page.next_before)
      }
      error.value = null
    } catch (e) {
      error.value = msg(e)
    } finally {
      pending.value--
    }
  }

  /** Appends the next page of a tab while its cursor is set. */
  async function loadMore(tab: TaskTab): Promise<void> {
    const before = nextBefore.value[tab]
    if (!before) return
    pending.value++
    try {
      const page = await api.tasks({ ...query(tab, memberIds(tab)[0]), before })
      setTab(tab, [...(tabs.value[tab] ?? []), ...page.items], page.next_before)
      error.value = null
    } catch (e) {
      error.value = msg(e)
    } finally {
      pending.value--
    }
  }

  async function loadRange(from: string, to: string): Promise<void> {
    pending.value++
    try {
      ranges.value[`${from}|${to}`] = await api.tasksAll({ from, to })
      error.value = null
    } catch (e) {
      error.value = msg(e)
    } finally {
      pending.value--
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
    nextBefore.value = {}
    ranges.value = {}
    wanted.clear()
    undo.value = null
  }

  return {
    tabs,
    nextBefore,
    ranges,
    error,
    loading,
    conflict,
    undo,
    filter,
    setFilter,
    validateFilter,
    load,
    loadMore,
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
