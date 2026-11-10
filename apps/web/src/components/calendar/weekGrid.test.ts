import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { EventLike, MemberLike } from '../../lib/types'
import WeekGrid from './WeekGrid.vue'
import { buildWeekGrid, EVERYONE } from './weekGrid'

const members: MemberLike[] = [
  { id: 'a', display_name: 'Alex', color: 'red' },
  { id: 'b', display_name: 'Bo', color: 'blue' },
]
const ev = (id: string, start: string, ms: string[], extra: Partial<EventLike> = {}): EventLike =>
  ({
    id,
    title: id,
    start_at: start,
    end_at: new Date(Date.parse(start) + 3_600_000).toISOString(),
    all_day: false,
    members: ms,
    color: 'red',
    ...extra,
  }) as EventLike
const base = { members, chosen: [], start: '2026-11-09', tz: 'America/New_York' }
const now = new Date('2026-11-10T15:00:00Z')

describe('buildWeekGrid', () => {
  it('has a column per member and 7 day rows, today flagged', () => {
    const g = buildWeekGrid({ ...base, events: [], now })
    expect(g.columns.map((c) => c.key)).toEqual(['a', 'b'])
    expect(g.rows.map((r) => r.key)[0]).toBe('2026-11-09')
    expect(g.rows).toHaveLength(7)
    expect(g.rows.filter((r) => r.today).map((r) => r.key)).toEqual(['2026-11-10'])
  })
  it('shows a multi-member event in each member column', () => {
    const g = buildWeekGrid({ ...base, events: [ev('x', '2026-11-10T15:00:00Z', ['a', 'b'])], now })
    const row = g.rows[1]!
    expect(row.cells.map((c) => c.events.length)).toEqual([1, 1])
  })
  it('hides columns of unchosen members', () => {
    const g = buildWeekGrid({
      ...base,
      chosen: ['b'],
      events: [ev('x', '2026-11-10T15:00:00Z', ['a', 'b']), ev('w', '2026-11-10T15:00:00Z', [])],
      now,
    })
    expect(g.columns.map((c) => c.key)).toEqual(['b'])
  })
  it('gives household-wide events an Everyone column while nobody is chosen', () => {
    const g = buildWeekGrid({ ...base, events: [ev('w', '2026-11-10T15:00:00Z', [])], now })
    expect(g.columns.at(-1)?.key).toBe(EVERYONE)
    expect(g.rows[1]!.cells.at(-1)!.events).toHaveLength(1)
  })
  it('buckets by household time zone, not UTC', () => {
    // 02:00 UTC on the 10th is the evening of the 9th in New York.
    const g = buildWeekGrid({ ...base, events: [ev('late', '2026-11-10T02:00:00Z', ['a'])], now })
    expect(g.rows[0]!.cells[0]!.events).toHaveLength(1)
    expect(g.rows[1]!.cells[0]!.events).toHaveLength(0)
  })
  it('starts on the given week start (sunday)', () => {
    const g = buildWeekGrid({ ...base, start: '2026-11-08', events: [], now })
    expect(g.rows[0]!.weekday).toBe('Sun')
  })
})

describe('WeekGrid and event style', () => {
  it('renders table semantics and a rounded multi-colour event', async () => {
    const w = mount(WeekGrid, {
      props: {
        ...base,
        events: [ev('x', '2026-11-10T15:00:00Z', ['a', 'b'], { colors: ['red', 'blue'] })],
      },
    })
    expect(w.findAll('th[scope="col"]')).toHaveLength(3)
    const btn = w.find('button.ev')
    const style = btn.attributes('style') ?? ''
    expect(style).not.toContain('border-image')
    expect(style).toContain('linear-gradient')
    expect(btn.classes()).not.toContain('square')
    await btn.trigger('click')
    expect(w.emitted('select')).toHaveLength(1)
  })
})
