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
    m.bills.mockResolvedValue([])
    const s = useBillsStore()
    s.filter = 'open'
    await s.load()
    expect(m.bills).toHaveBeenCalledWith('open')
    s.filter = ''
    await s.load()
    expect(m.bills).toHaveBeenLastCalledWith(undefined)
  })

  it('patch reloads, and a conflict reloads and reports false', async () => {
    m.bills.mockResolvedValue([])
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
