import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { TaskView } from '../../api/client'
import { addDays, dayKey } from '../../lib/time'
import { dailyRows, hoursOf, mealCards, tasksDueOn } from './todayModel'
import TodayDay from './TodayDay.vue'
import TodayWeather from './TodayWeather.vue'

const entry = (id: string, slot: string, ord: number, title: string, recipe?: string) =>
  ({
    id,
    date: '2026-11-10',
    slot_id: slot,
    ord,
    title,
    recipe_id: recipe ?? null,
    version: 1,
  }) as never

describe('todayModel', () => {
  it('groups meals per slot in slot order and drops empty slots', () => {
    const cards = mealCards(
      [
        entry('a', 's2', 1, 'Pasta', 'r1'),
        entry('b', 's2', 0, 'Salad'),
        entry('c', 's1', 0, 'Oats'),
      ],
      '2026-11-10',
      [
        { id: 's1', name: 'Breakfast' },
        { id: 's0', name: 'Lunch' },
        { id: 's2', name: 'Dinner' },
      ],
    )
    expect(cards.map((c) => c.slot)).toEqual(['Breakfast', 'Dinner'])
    expect(cards[1]!.items.map((i) => i.title)).toEqual(['Salad', 'Pasta'])
    expect(cards[1]!.items[1]!.recipeId).toBe('r1')
  })

  it('picks tasks due on the household day', () => {
    const t = (id: string, due: string | null, status = 'open') =>
      ({ id, due_at: due, status }) as TaskView
    const got = tasksDueOn(
      [
        t('a', '2026-11-11T15:00:00Z'),
        t('b', '2026-11-12T15:00:00Z'),
        t('c', null),
        t('d', '2026-11-11T15:00:00Z', 'done'),
      ],
      '2026-11-11',
      'America/New_York',
    )
    expect(got.map((x) => x.id)).toEqual(['a'])
  })

  it('reads hours of one day and the daily rows', () => {
    const hourly = {
      time: ['2026-11-10T23:00', '2026-11-11T00:00', '2026-11-11T01:00'],
      temperature_2m: [5, 4, 3],
      weather_code: [0, 1, 2],
      precipitation_probability: [0, 10, 20],
    }
    const h = hoursOf(hourly, '2026-11-11')
    expect(h.map((x) => x.time)).toEqual(['00:00', '01:00'])
    expect(h[1]!.rain).toBe(20)
    expect(dailyRows({ time: ['2026-11-11'], temperature_2m_max: [9] })[0]!.hi).toBe(9)
  })
})

describe('TodayDay', () => {
  const base = {
    tz: 'America/New_York',
    meals: [
      {
        slot: 'Dinner',
        items: [
          { id: 'a', title: 'Pasta', recipeId: 'r1' },
          { id: 'b', title: 'Salad', recipeId: null },
        ],
      },
    ],
    tasks: [],
  }
  const ev = (location: string | null) =>
    ({
      id: 'e1',
      title: 'Practice',
      location,
      start_at: '2026-11-10T22:00:00Z',
      end_at: '2026-11-10T23:00:00Z',
      all_day: false,
      members: [],
      color: 'blue',
      visibility: 'full',
    }) as Record<string, unknown>

  it('titles a card with the slot and links recipes to their page', () => {
    const w = mount(TodayDay, {
      props: { ...base, events: [] },
      global: { stubs: { RouterLink: { props: ['to'], template: '<a :href="to"><slot /></a>' } } },
    })
    expect(w.get('[data-testid=meal-card] h2').text()).toBe('Dinner')
    expect(w.get('[data-testid=meal-recipe]').attributes('href')).toBe('/food/recipes/r1')
    expect(w.findAll('[data-testid=meal-item]')).toHaveLength(2)
  })

  it('shows a map link for events with a location only', () => {
    const w = mount(TodayDay, {
      props: { ...base, meals: [], events: [ev('1 Main St'), { ...ev(null), id: 'e2' }] as never },
    })
    const links = w.findAll('[data-testid=event-map]')
    expect(links).toHaveLength(1)
    expect(links[0]!.attributes('href')).toContain('1%20Main%20St')
  })
})

describe('TodayWeather', () => {
  const today = dayKey(new Date(), 'UTC')
  const weather = {
    current: { temperature_2m: 8, weather_code: 0, is_day: 1 },
    daily: {
      time: [today, addDays(today, 1), addDays(today, 2)],
      temperature_2m_max: [10, 11, 12],
      temperature_2m_min: [1, 2, 3],
      weather_code: [0, 1, 2],
    },
    hourly: { time: [`${today}T09:00`], temperature_2m: [7], weather_code: [0] },
  } as never
  const mountIt = (mode: 'today' | 'tomorrow') =>
    mount(TodayWeather, {
      props: { weather, tz: 'UTC', units: 'metric' as never, mode },
      attachTo: document.body,
    })

  it('lists the remaining days for tomorrow', () => {
    const w = mountIt('tomorrow')
    expect(w.findAll('[data-testid=forecast-day]')).toHaveLength(2)
    w.unmount()
  })

  it('opens the hourly breakdown on click', async () => {
    const w = mountIt('today')
    expect(document.body.querySelectorAll('[data-testid=hour-row]')).toHaveLength(0)
    await w.get('[data-testid=today-weather]').trigger('click')
    expect(document.body.querySelectorAll('[data-testid=hour-row]')).toHaveLength(1)
    w.unmount()
  })
})
