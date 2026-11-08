import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { createOfflineQueue, QUEUE_KEY, type QueueOp } from './offlineQueue'

afterEach(() => {
  vi.restoreAllMocks()
  localStorage.clear()
})

const add = (id: string): QueueOp => ({ kind: 'add', id, body: { id, text: id } })
const apiErr = (status: number) => new ApiError(status, {})

describe('offlineQueue', () => {
  it('replays in order and empties', async () => {
    const q = createOfflineQueue()
    q.enqueue(add('a'))
    q.enqueue({ kind: 'check', id: 'a' })
    q.enqueue(add('b'))
    expect(q.pending.value).toBe(3)
    const seen: string[] = []
    await q.flush(async (op) => {
      seen.push(`${op.kind}:${op.id}`)
    })
    expect(seen).toEqual(['add:a', 'check:a', 'add:b'])
    expect(q.pending.value).toBe(0)
  })

  it('persists across instances and clears storage when empty', async () => {
    const q = createOfflineQueue()
    q.enqueue(add('a'))
    expect(JSON.parse(localStorage.getItem(QUEUE_KEY)!)).toHaveLength(1)
    const again = createOfflineQueue()
    expect(again.pending.value).toBe(1)
    await again.flush(async () => {})
    expect(localStorage.getItem(QUEUE_KEY)).toBeNull()
  })

  it('stops at the first network error and goes offline', async () => {
    const q = createOfflineQueue()
    q.enqueue(add('a'))
    q.enqueue(add('b'))
    const send = vi.fn().mockRejectedValueOnce(new TypeError('Failed to fetch'))
    await q.flush(send)
    expect(send).toHaveBeenCalledTimes(1)
    expect(q.pending.value).toBe(2)
    expect(q.online.value).toBe(false)
    send.mockResolvedValue(undefined)
    await q.flush(send)
    expect(q.pending.value).toBe(0)
    expect(q.online.value).toBe(true)
  })

  it('drops an op on 404 and carries on', async () => {
    const q = createOfflineQueue()
    q.enqueue({ kind: 'check', id: 'gone' })
    q.enqueue({ kind: 'check', id: 'there' })
    const send = vi.fn().mockRejectedValueOnce(apiErr(404)).mockResolvedValue(undefined)
    await q.flush(send)
    expect(send).toHaveBeenCalledTimes(2)
    expect(q.pending.value).toBe(0)
  })

  it('counts a 409 on check as done, but keeps an op on a 5xx', async () => {
    const q = createOfflineQueue()
    q.enqueue({ kind: 'check', id: 'x' })
    await q.flush(vi.fn().mockRejectedValue(apiErr(409)))
    expect(q.pending.value).toBe(0)
    q.enqueue(add('y'))
    await q.flush(vi.fn().mockRejectedValue(apiErr(503)))
    expect(q.pending.value).toBe(1)
  })

  it('falls back to memory when localStorage throws', async () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    vi.spyOn(Storage.prototype, 'removeItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    const q = createOfflineQueue()
    q.enqueue(add('a'))
    expect(q.pending.value).toBe(1)
    expect(q.ops()).toHaveLength(1)
    await q.flush(async () => {})
    expect(q.pending.value).toBe(0)
  })
})
