import { defineStore } from 'pinia'
import { computed, ref, shallowRef, watch } from 'vue'
import {
  food,
  type FoodApi,
  type GroceryAddReq,
  type GroceryItemPatch,
  type GroceryItemView,
  type GroceryView,
  type StapleView,
  type StoreAisleView,
} from '../api/food'
import { isConflict } from '../api/http'
import { newId } from '../lib/ids'
import { groceryQueue, isNetworkError, type QueueOp } from '../lib/offlineQueue'

const UNDO_DELETE_MS = 5000
const RETRY_MS = 15_000

/** The household's one living list (D-021), with optimistic changes and the offline queue (D-118). */
export const useGroceryStore = defineStore('grocery', () => {
  const api = shallowRef<FoodApi>(food)
  const queue = groceryQueue
  const view = ref<GroceryView | null>(null)
  /** Per-store aisle order and hidden flags (D-121); the view itself carries the household order. */
  const storeAisles = ref<Record<string, StoreAisleView[]>>({})
  const loaded = ref(false)
  const error = ref<string | null>(null)
  /** Items removed locally while their Undo window is open. */
  const hidden = ref<Set<string>>(new Set())
  const deleteTimers = new Map<string, ReturnType<typeof setTimeout>>()

  const items = computed(() => (view.value?.items ?? []).filter((i) => !hidden.value.has(i.id)))
  const open = computed(() => items.value.filter((i) => !i.checked_at && i.section === 'list'))
  const checked = computed(() =>
    items.value
      .filter((i) => i.checked_at)
      .sort((a, b) => (b.checked_at ?? '').localeCompare(a.checked_at ?? '')),
  )
  const staples = computed(() => view.value?.staples ?? [])
  const stores = computed(() => view.value?.stores ?? [])
  const aisles = computed(() => view.value?.aisles ?? [])
  const online = queue.online
  const pending = queue.pending

  /** Which API to use: the app's, or the kiosk's device-token one. */
  function use(a: FoodApi): void {
    api.value = a
  }

  async function loadStoreAisles(v: GroceryView): Promise<void> {
    const ids = v.stores.map((s) => s.id).filter((id): id is NonNullable<typeof id> => !!id)
    const got = await Promise.allSettled(ids.map((id) => api.value.storeAisles(id)))
    const next: Record<string, StoreAisleView[]> = {}
    got.forEach((r, i) => {
      if (r.status === 'fulfilled') next[ids[i]!] = r.value
      else if (storeAisles.value[ids[i]!]) next[ids[i]!] = storeAisles.value[ids[i]!]!
    })
    storeAisles.value = next
  }

  async function load(): Promise<void> {
    // Local changes still waiting would be wiped by a refetch: replay them first, and keep the
    // optimistic view while they cannot be sent.
    if (queue.pending.value > 0) {
      await queue.flush(sendOp)
      if (queue.pending.value > 0) {
        loaded.value = true
        return
      }
    }
    try {
      const v = await api.value.grocery()
      view.value = v
      error.value = null
      queue.markOnline()
      await loadStoreAisles(v)
    } catch (e) {
      if (isNetworkError(e)) queue.markOffline()
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  function sendOp(op: QueueOp): Promise<unknown> {
    const a = api.value
    switch (op.kind) {
      case 'add':
        return a.addItem(op.body as GroceryAddReq)
      case 'check':
        return a.checkItem(op.id)
      case 'uncheck':
        return a.uncheckItem(op.id)
      case 'low':
        return a.lowItem(op.id)
    }
  }

  /** Sends now, or queues when offline or when older changes are still waiting (order matters). */
  async function dispatch(op: QueueOp): Promise<GroceryItemView | null> {
    if (!queue.online.value || queue.pending.value > 0) {
      queue.enqueue(op)
      if (queue.online.value) void queue.flush(sendOp).then(() => load())
      return null
    }
    try {
      return (await sendOp(op)) as GroceryItemView
    } catch (e) {
      if (isNetworkError(e)) {
        queue.markOffline()
        queue.enqueue(op)
        return null
      }
      throw e
    }
  }

  function replace(item: GroceryItemView): void {
    if (!view.value) return
    const i = view.value.items.findIndex((x) => x.id === item.id)
    if (i >= 0) view.value.items[i] = item
    else view.value.items.push(item)
  }

  async function revert(e: unknown): Promise<never> {
    await load()
    throw e
  }

  /** The id is made here, so a replay after a lost answer is idempotent (D-118). */
  async function add(text: string, opts: { qty?: number; unit?: string; store_id?: string } = {}) {
    const t = text.trim()
    if (!t || !view.value) return
    const id = newId()
    const body = { id, text: t, ...opts } as GroceryAddReq
    view.value.items.push({
      id,
      text: t,
      display: opts.qty != null ? `${opts.qty}${opts.unit ? ` ${opts.unit}` : ''}` : '',
      qty: opts.qty,
      unit: opts.unit,
      store_id: opts.store_id,
      section: 'list',
      manual: true,
      sources: [],
      version: 0,
    } as GroceryItemView)
    try {
      const got = await dispatch({ kind: 'add', id, body })
      if (got) replace(got)
    } catch (e) {
      await revert(e)
    }
  }

  async function setChecked(item: GroceryItemView, on: boolean) {
    const prev = item.checked_at
    item.checked_at = on ? new Date().toISOString() : null
    try {
      const got = await dispatch({ kind: on ? 'check' : 'uncheck', id: item.id })
      if (got) replace(got)
    } catch (e) {
      item.checked_at = prev
      await revert(e)
    }
  }
  const check = (item: GroceryItemView) => setChecked(item, true)
  const uncheck = (item: GroceryItemView) => setChecked(item, false)

  /** "Low" on a staple: it moves to the list (the staple's item becomes a list item). */
  async function low(staple: StapleView) {
    if (!view.value) return
    view.value.staples = view.value.staples.filter((s) => s.item_id !== staple.item_id)
    view.value.items.push({
      id: staple.item_id,
      text: staple.name,
      display: '',
      ingredient_id: staple.ingredient_id,
      section: 'list',
      manual: false,
      sources: staple.needed_for,
      version: 0,
    } as GroceryItemView)
    try {
      const got = await dispatch({ kind: 'low', id: staple.item_id })
      if (got) replace(got)
    } catch (e) {
      await revert(e)
    }
  }

  /** Adults only, online only. Returns false on a stale version (the list is reloaded). */
  async function update(item: GroceryItemView, patch: GroceryItemPatch): Promise<boolean> {
    try {
      replace(await api.value.updateItem(item.id, item.version, patch))
      return true
    } catch (e) {
      if (isConflict(e)) {
        await load()
        return false
      }
      throw e
    }
  }

  /** Adults only, online only: hidden at once, deleted when the Undo window closes. */
  function remove(item: GroceryItemView, onFailed?: (e: unknown) => void): () => void {
    hidden.value = new Set(hidden.value).add(item.id)
    const timer = setTimeout(async () => {
      deleteTimers.delete(item.id)
      try {
        await api.value.deleteItem(item.id)
      } catch (e) {
        // 404: already gone, which is what was wanted.
        if (!(e && typeof e === 'object' && 'status' in e && e.status === 404)) onFailed?.(e)
      } finally {
        const next = new Set(hidden.value)
        next.delete(item.id)
        hidden.value = next
        void load()
      }
    }, UNDO_DELETE_MS)
    deleteTimers.set(item.id, timer)
    return () => {
      clearTimeout(timer)
      deleteTimers.delete(item.id)
      const next = new Set(hidden.value)
      next.delete(item.id)
      hidden.value = next
    }
  }

  /** Undoes a generate or reconcile (`POST /meals/changes/{id}/undo`). */
  async function undo(changeId: string): Promise<void> {
    await api.value.undo(changeId)
    await load()
  }

  async function invalidate(kind: string): Promise<void> {
    if (!loaded.value) return
    if (kind === 'grocery.updated' || kind === 'resync') await load()
  }

  // Back online: replay the queue, then refetch.
  watch(
    () => queue.online.value,
    (on) => {
      if (on && loaded.value) void load()
    },
  )

  // The browser may report online while the server is out of reach, so no `online` event comes:
  // retry now and then while changes wait.
  let retry: ReturnType<typeof setInterval> | undefined
  watch(
    () => queue.pending.value,
    (n) => {
      if (n > 0 && !retry) retry = setInterval(() => void load(), RETRY_MS)
      else if (n === 0 && retry) {
        clearInterval(retry)
        retry = undefined
      }
    },
    { immediate: true },
  )

  return {
    view,
    storeAisles,
    loaded,
    error,
    items,
    open,
    checked,
    staples,
    stores,
    aisles,
    online,
    pending,
    use,
    load,
    add,
    check,
    uncheck,
    low,
    update,
    remove,
    undo,
    invalidate,
  }
})
