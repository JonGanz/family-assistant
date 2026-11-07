import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { formatMoney, useBillsStore } from './bills'

vi.mock('../api/client', () => ({ api: { bills: vi.fn(), patchBill: vi.fn() } }))
import { api } from '../api/client'
const m = vi.mocked(api)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
})

describe('formatMoney', () => {
  it('uses the currency minor digits', () => {
    expect(formatMoney(12345, 'USD', 'en-US')).toBe('$123.45')
    expect(formatMoney(5000, 'JPY', 'en-US')).toContain('5,000')
  })
  it('survives an unknown currency code', () => {
    expect(formatMoney(100, 'xx', 'en-US')).toBe('1.00 xx')
  })
})

describe('bills store', () => {
  it('filters by status', async () => {
    m.bills.mockResolvedValue({ items: [] })
    const s = useBillsStore()
    s.filter = ['open']
    await s.load()
    expect(m.bills).toHaveBeenCalledWith('open')
    s.filter = []
    await s.load()
    expect(m.bills).toHaveBeenLastCalledWith(undefined)
  })

  it('sends no status for several chosen states', async () => {
    m.bills.mockResolvedValue({ items: [] })
    const s = useBillsStore()
    s.filter = ['open', 'paid']
    await s.load()
    expect(m.bills).toHaveBeenCalledWith(undefined)
  })

  it('loads more with the cursor and keeps the soonest due first', async () => {
    m.bills.mockResolvedValueOnce({
      items: [{ id: 'b1', due_on: '2026-12-01' }],
      next_before: '2026-11-01T00:00:00Z',
    } as never)
    m.bills.mockResolvedValueOnce({ items: [{ id: 'b2', due_on: '2026-11-15' }] } as never)
    const s = useBillsStore()
    await s.load()
    expect(s.nextBefore).toBe('2026-11-01T00:00:00Z')
    await s.loadMore()
    expect(m.bills).toHaveBeenLastCalledWith(undefined, '2026-11-01T00:00:00Z')
    expect(s.items.map((b) => b.id)).toEqual(['b2', 'b1'])
    expect(s.nextBefore).toBeNull()
  })

  it('patch reloads, and a conflict reloads and reports false', async () => {
    m.bills.mockResolvedValue({ items: [] })
    m.patchBill.mockResolvedValueOnce({} as never)
    const s = useBillsStore()
    const bill = { id: 'b', version: 3 } as never
    expect(await s.patch(bill, { status: 'dismissed' })).toBe(true)
    expect(m.patchBill).toHaveBeenCalledWith('b', 3, { status: 'dismissed' })
    m.patchBill.mockRejectedValueOnce(new ApiError(409, {}))
    expect(await s.patch(bill, { status: 'open' })).toBe(false)
    expect(m.bills).toHaveBeenCalledTimes(2)
  })
})
