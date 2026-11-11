import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { itemId, useInboxStore } from './inbox'

vi.mock('../api/client', () => ({
  api: {
    inbox: vi.fn(),
    approveInbox: vi.fn(),
    rejectInbox: vi.fn(),
    discardInbox: vi.fn(),
    discardPastEvents: vi.fn(),
  },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

const view = {
  items: [
    { kind: 'event', event: { id: 'e1' } },
    { kind: 'bill', bill: { id: 'b1' } },
  ],
  counts: { event: 1, bill: 1, task: 0, contact: 0, total: 2 },
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  m.inbox.mockResolvedValue(view as never)
})

describe('inbox store', () => {
  it('loadMore appends the next page', async () => {
    m.inbox.mockResolvedValueOnce({ ...view, items: [view.items[0]], next_before: 'c1' } as never)
    m.inbox.mockResolvedValueOnce({ ...view, items: [view.items[1]] } as never)
    const s = useInboxStore()
    await s.load()
    expect(s.nextBefore).toBe('c1')
    await s.loadMore()
    expect(m.inbox).toHaveBeenLastCalledWith(undefined, 'c1')
    expect(s.items).toHaveLength(2)
    expect(s.nextBefore).toBeNull()
  })

  it('loads items and counts', async () => {
    const s = useInboxStore()
    await s.load()
    expect(s.items).toHaveLength(2)
    expect(s.total).toBe(2)
    expect(itemId(s.items[1]!)).toBe('b1')
  })

  it('approve removes optimistically and sends the edits', async () => {
    m.approveInbox.mockResolvedValue(undefined as never)
    const s = useInboxStore()
    await s.load()
    const ok = await s.approve('event', 'e1', { title: 'New' })
    expect(ok).toBe(true)
    expect(m.approveInbox).toHaveBeenCalledWith('event', 'e1', { title: 'New' })
    expect(s.items).toHaveLength(1)
    expect(s.counts.event).toBe(0)
    expect(s.total).toBe(1)
  })

  it('rolls back when the server refuses', async () => {
    m.rejectInbox.mockRejectedValue(new Error('stale'))
    const s = useInboxStore()
    await s.load()
    const ok = await s.reject('bill', 'b1')
    expect(ok).toBe(false)
    expect(s.items).toHaveLength(2)
    expect(s.total).toBe(2)
    expect(s.error).toBe('stale')
  })

  it('discard removes optimistically and calls the discard endpoint, not reject', async () => {
    m.discardInbox.mockResolvedValue(undefined as never)
    const s = useInboxStore()
    await s.load()
    expect(await s.discard('bill', 'b1')).toBe(true)
    expect(m.discardInbox).toHaveBeenCalledWith('bill', 'b1')
    expect(m.rejectInbox).not.toHaveBeenCalled()
    expect(s.items).toHaveLength(1)
    expect(s.counts.bill).toBe(0)
  })

  it('discardPast reloads and returns the count', async () => {
    m.discardPastEvents.mockResolvedValue({ count: 3 })
    const s = useInboxStore()
    expect(await s.discardPast()).toBe(3)
    expect(m.inbox).toHaveBeenCalled()
    m.discardPastEvents.mockRejectedValue(new Error('nope'))
    expect(await s.discardPast()).toBeNull()
    expect(s.error).toBe('nope')
  })

  it('inbox.updated refetches once loaded, other kinds do not', async () => {
    const s = useInboxStore()
    await s.invalidate('inbox.updated')
    expect(m.inbox).not.toHaveBeenCalled()
    await s.load()
    m.inbox.mockClear()
    await s.invalidate('tasks.updated')
    expect(m.inbox).not.toHaveBeenCalled()
    await s.invalidate('inbox.updated')
    expect(m.inbox).toHaveBeenCalledTimes(1)
  })
})
