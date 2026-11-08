import { ref, type Ref } from 'vue'
import { ApiError } from '../api/http'

/** One queued grocery change (D-118). `id` is the item id; `add` carries the full request. */
export interface QueueOp {
  kind: 'add' | 'check' | 'uncheck' | 'low'
  id: string
  body?: unknown
}

export const QUEUE_KEY = 'fa.grocery.queue'

export interface OfflineQueue {
  /** The number of changes waiting. */
  pending: Ref<number>
  /** `navigator.onLine`, the `online`/`offline` events, and failed requests. */
  online: Ref<boolean>
  enqueue(op: QueueOp): void
  /** A copy of the waiting ops, oldest first. */
  ops(): QueueOp[]
  /** Replays in order. A network error stops it (and marks offline); a 404 drops the op. */
  flush(send: (op: QueueOp) => Promise<unknown>): Promise<void>
  markOffline(): void
  markOnline(): void
}

/** A request that never got an answer (offline, DNS, refused): not an HTTP problem response. */
export function isNetworkError(e: unknown): boolean {
  return !(e instanceof ApiError)
}

function readStored(key: string): QueueOp[] {
  try {
    const raw = localStorage.getItem(key)
    const v: unknown = raw ? JSON.parse(raw) : []
    return Array.isArray(v) ? (v as QueueOp[]).filter((o) => o && typeof o.id === 'string') : []
  } catch {
    return []
  }
}

/**
 * A small queue kept in `localStorage` (every access wrapped), with an in-memory copy that keeps
 * working when storage throws (private windows, blocked site data).
 */
export function createOfflineQueue(key: string = QUEUE_KEY): OfflineQueue {
  let list: QueueOp[] = readStored(key)
  const pending = ref(list.length)
  const online = ref(typeof navigator === 'undefined' ? true : navigator.onLine !== false)
  let flushing: Promise<void> | null = null

  function persist() {
    pending.value = list.length
    try {
      if (list.length) localStorage.setItem(key, JSON.stringify(list))
      else localStorage.removeItem(key)
    } catch {
      /* in-memory only */
    }
  }

  if (typeof window !== 'undefined') {
    window.addEventListener('online', () => (online.value = true))
    window.addEventListener('offline', () => (online.value = false))
  }

  async function run(send: (op: QueueOp) => Promise<unknown>) {
    while (list.length) {
      const op = list[0]!
      try {
        await send(op)
        online.value = true
      } catch (e) {
        if (isNetworkError(e)) {
          online.value = false
          return
        }
        const status = (e as ApiError).status
        // 404: the item is gone. 409 on a check: check-offs win, so it counts as done. Any other
        // 4xx can never succeed on replay. A 5xx may, so it stays queued.
        const drop =
          status === 404 ||
          (status === 409 && op.kind === 'check') ||
          (status >= 400 && status < 500 && status !== 408 && status !== 429)
        if (!drop) return
      }
      list = list.slice(1)
      persist()
    }
  }

  return {
    pending,
    online,
    enqueue(op) {
      list = [...list, op]
      persist()
    },
    ops: () => list.map((o) => ({ ...o })),
    flush(send) {
      flushing ??= run(send).finally(() => {
        flushing = null
      })
      return flushing
    },
    markOffline() {
      online.value = false
    },
    markOnline() {
      online.value = true
    },
  }
}

/** The grocery list's queue. */
export const groceryQueue: OfflineQueue = createOfflineQueue()
