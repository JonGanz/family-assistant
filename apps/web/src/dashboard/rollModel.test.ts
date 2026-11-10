import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const plan = vi.hoisted(() => vi.fn())
vi.mock('../api/food', async (orig) => {
  const real = await orig<typeof import('../api/food')>()
  const kiosk = { plan, slots: vi.fn().mockResolvedValue([]) }
  return { ...real, foodKiosk: kiosk, food: kiosk }
})

import FoodPlanRoll from './FoodPlanRoll.vue'
import { dayKey } from '../lib/time'
import { rollingDays } from './rollModel'

describe('rollingDays', () => {
  it('puts today third: two before, four after', () => {
    const d = rollingDays('2026-11-09')
    expect(d).toEqual([
      '2026-11-07',
      '2026-11-08',
      '2026-11-09',
      '2026-11-10',
      '2026-11-11',
      '2026-11-12',
      '2026-11-13',
    ])
    expect(d[2]).toBe('2026-11-09')
  })
  it('crosses month and year boundaries', () => {
    expect(rollingDays('2026-03-01')[0]).toBe('2026-02-27')
    expect(rollingDays('2026-12-30').at(-1)).toBe('2027-01-03')
    expect(rollingDays('2028-03-01')[0]).toBe('2028-02-28')
  })
  it('follows the household zone for today', () => {
    const now = new Date('2026-11-09T23:30:00Z')
    expect(rollingDays(dayKey(now, 'UTC'))[2]).toBe('2026-11-09')
    expect(rollingDays(dayKey(now, 'Pacific/Auckland'))[2]).toBe('2026-11-10')
  })
})

describe('FoodPlanRoll', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    plan.mockReset()
    plan.mockResolvedValue([
      {
        id: 'e1',
        date: '2026-11-09',
        slot_id: 's1',
        ord: 0,
        title: 'Lentil soup',
        free_text: null,
        recipe_id: null,
      },
    ])
  })
  it('loads today-2 .. today+4 and marks the third row as today', async () => {
    const w = mount(FoodPlanRoll, {
      props: { today: '2026-11-09', slots: [{ id: 's1', name: 'Dinner' }] },
    })
    await flushPromises()
    expect(plan).toHaveBeenCalledWith('2026-11-07', '2026-11-13')
    const rows = w.findAll('tbody tr')
    expect(rows.map((r) => r.attributes('data-day'))).toEqual(rollingDays('2026-11-09'))
    expect(rows[2]!.classes()).toContain('today')
    expect(rows.filter((r) => r.classes().includes('today'))).toHaveLength(1)
    expect(w.find('[data-testid="roll-cell-2026-11-09-dinner"]').text()).toContain('Lentil soup')
  })
})
