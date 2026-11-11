import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DateRangeNav from '../components/DateRangeNav.vue'
import EventCard from '../components/EventCard.vue'
import { dayKey } from '../lib/time'
import CalendarView from './CalendarView.vue'

const day = dayKey(new Date(), 'UTC')
const mk = (i: number, location?: string) => ({
  id: `e${i}`,
  title: `Event ${i}`,
  start_at: `${day}T0${i}:00:00Z`,
  end_at: `${day}T0${i}:30:00Z`,
  all_day: false,
  members: [],
  location,
  visibility: 'full',
  color: null,
})

vi.mock('../api/client', () => ({
  api: new Proxy(
    {},
    {
      get: (_t, name) =>
        name === 'events'
          ? async () => [1, 2, 3, 4, 5].map((i) => mk(i, i === 5 ? 'Town Hall' : undefined))
          : async () => [],
    },
  ),
}))

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.setItem('fa.calendar.view', 'month')
})

describe('DateRangeNav', () => {
  it('uses chevrons for both directions', () => {
    const w = mount(DateRangeNav, { props: { label: 'x' } })
    const [prev, next] = w.findAll('button')
    expect(prev!.attributes('aria-label')).toBe('Previous')
    expect(prev!.html()).not.toEqual(next!.html().replace('Next', 'Previous'))
    expect(next!.attributes('aria-label')).toBe('Next')
  })
})

describe('EventCard location', () => {
  const ev = { ...mk(1, 'Town Hall') } as never
  it('links to a map in a new tab on desktop', () => {
    const w = mount(EventCard, { props: { event: ev, tz: 'UTC' } })
    const a = w.find('a.map-link')
    expect(a.attributes('href')).toContain('Town%20Hall')
    expect(a.attributes('target')).toBe('_blank')
    expect(a.attributes('rel')).toBe('noopener')
  })
  it('has no link without a location', () => {
    const w = mount(EventCard, { props: { event: mk(1) as never, tz: 'UTC' } })
    expect(w.find('a.map-link').exists()).toBe(false)
  })
})

describe('CalendarView month overflow', () => {
  it('opens the day list, then an event with a back button', async () => {
    const w = mount(CalendarView, { attachTo: document.body, global: { stubs: { UiPage: false } } })
    await flushPromises()
    const more = w.find('button.more')
    expect(more.text()).toContain('+2 more')
    await more.trigger('click')
    const body = () => document.body.textContent ?? ''
    expect(body()).toContain('Event 5')
    expect(document.body.querySelector('.day-list')).not.toBeNull()
    const card = [...document.body.querySelectorAll('.day-list [role=button]')].find((c) =>
      c.textContent?.includes('Event 5'),
    ) as HTMLElement
    card.click()
    await flushPromises()
    expect(document.body.querySelector('.day-list')).toBeNull()
    expect(document.body.querySelector('a.map-link')).not.toBeNull()
    const back = [...document.body.querySelectorAll('button')].find((b) =>
      b.textContent?.includes('Back to'),
    ) as HTMLElement
    back.click()
    await flushPromises()
    expect(document.body.querySelector('.day-list')).not.toBeNull()
    w.unmount()
  })
})

describe('CalendarView week overflow', () => {
  it('"+N events" opens the day list, drills into an event and back', async () => {
    localStorage.setItem('fa.calendar.view', 'week')
    const w = mount(CalendarView, { attachTo: document.body, global: { stubs: { UiPage: false } } })
    await flushPromises()
    const more = w.find('button.more')
    expect(more.text()).toBe('+3 events')
    await more.trigger('click')
    expect(document.body.querySelector('.day-list')).not.toBeNull()
    const card = [...document.body.querySelectorAll('.day-list [role=button]')].find((c) =>
      c.textContent?.includes('Event 5'),
    ) as HTMLElement
    card.click()
    await flushPromises()
    expect(document.body.querySelector('.day-list')).toBeNull()
    const back = [...document.body.querySelectorAll('button')].find((b) =>
      b.textContent?.includes('Back to'),
    ) as HTMLElement
    back.click()
    await flushPromises()
    expect(document.body.querySelector('.day-list')).not.toBeNull()
    w.unmount()
  })
})
