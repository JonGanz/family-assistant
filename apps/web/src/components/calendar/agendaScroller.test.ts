import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { dayKey } from '../../lib/time'
import { useCalendarStore } from '../../stores/calendar'
import AgendaScroller from './AgendaScroller.vue'

const calls: string[] = []
vi.mock('../../api/client', () => ({
  api: {
    events: async (from: string) => {
      calls.push(from)
      return [
        {
          id: `e-${from}`,
          title: `Event ${from.slice(0, 10)}`,
          start_at: from,
          end_at: from,
          all_day: false,
          members: [],
          visibility: 'full',
          color: null,
        },
      ]
    },
  },
}))

let cbs: ((e: Partial<IntersectionObserverEntry>[]) => void)[] = []
let observed: Element[] = []
let disconnected = 0
beforeEach(() => {
  setActivePinia(createPinia())
  calls.length = 0
  cbs = []
  observed = []
  disconnected = 0
  window.scrollTo = vi.fn() as never
  vi.stubGlobal(
    'IntersectionObserver',
    class {
      constructor(cb: (e: Partial<IntersectionObserverEntry>[]) => void) {
        cbs.push(cb)
      }
      observe(el: Element) {
        observed.push(el)
      }
      disconnect() {
        disconnected++
      }
    },
  )
})

describe('AgendaScroller', () => {
  it('starts at the start day, loads on sentinels, keeps at most 5 chunks, cleans up', async () => {
    const start = dayKey(new Date(), 'UTC')
    const w = mount(AgendaScroller, {
      props: { tz: 'UTC', start, members: [], memberKeys: [undefined] },
    })
    await flushPromises()
    expect(calls).toHaveLength(1)
    expect(w.find('[data-day]').attributes('data-day')).toBe(start)
    expect(w.text()).toContain('Today')

    const bottom = observed.find((e) => (e as HTMLElement).dataset.edge === 'after')!
    const top = observed.find((e) => (e as HTMLElement).dataset.edge === 'before')!
    // Bounded by the chain guard in jsdom (every rect is zero, so sentinels look "near").
    cbs[0]!([{ isIntersecting: true, target: bottom }])
    await flushPromises()
    expect(calls.length).toBeGreaterThan(1)
    cbs[0]!([{ isIntersecting: true, target: top }])
    await flushPromises()
    cbs[0]!([{ isIntersecting: true, target: bottom }])
    await flushPromises()
    const calendar = useCalendarStore()
    expect(calendar.held.length).toBeLessThanOrEqual(5)

    w.unmount()
    expect(disconnected).toBe(1)
    expect(calendar.held).toHaveLength(0)
  })
})
