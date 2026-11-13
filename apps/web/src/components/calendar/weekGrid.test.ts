import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { EventLike, MemberLike } from '../../lib/types'
import WeekGrid from './WeekGrid.vue'
import { buildWeekGrid, EVERYONE, weekVisible } from './weekGrid'

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

  const three = [1, 2, 3, 4].map((i) => ev(`e${i}`, `2026-11-10T1${i}:00:00Z`, ['a']))
  it('shows at most 2 events plus "+N events" and a fixed row height', () => {
    const w = mount(WeekGrid, { props: { ...base, events: three } })
    const cell = w.find('tr[data-day="2026-11-10"] td[data-member="a"]')
    expect(cell.findAll('button.ev')).toHaveLength(2)
    expect(cell.find('button.more').text()).toBe('+2 events')
    expect(w.find('table').html()).toBeTruthy()
    expect(w.findAll('tbody tr')).toHaveLength(7)
    const css = readFileSync(resolve(__dirname, 'WeekGrid.vue'), 'utf8')
    expect(css).toContain('--wg-row-h')
    expect(css).toMatch(/tbody tr \{\s*height: var\(--wg-row-h\)/)
  })
  it('shows no "+N" with two events', () => {
    const w = mount(WeekGrid, { props: { ...base, events: three.slice(0, 2) } })
    expect(w.find('button.more').exists()).toBe(false)
  })
  it('puts the bold title before the time, single line', () => {
    const w = mount(WeekGrid, { props: { ...base, events: three.slice(0, 1) } })
    const spans = w.findAll('button.ev span').map((s) => s.classes()[0])
    expect(spans).toEqual(['title', 'time'])
    const css = readFileSync(resolve(__dirname, 'WeekGrid.vue'), 'utf8')
    expect(css).toMatch(/\.title \{[^}]*text-overflow: ellipsis[^}]*white-space: nowrap/)
  })
  it('emits more with the day and that column events', async () => {
    const w = mount(WeekGrid, { props: { ...base, events: three } })
    await w.find('button.more').trigger('click')
    const [p] = w.emitted('more')![0] as [{ day: string; events: unknown[] }]
    expect(p.day).toBe('2026-11-10')
    expect(p.events).toHaveLength(4)
  })
})

describe('weekVisible', () => {
  it('shows two on wide screens, the rest collapse', () => {
    expect([0, 1, 2, 3, 9].map((n) => weekVisible(n, false))).toEqual([0, 1, 2, 2, 2])
  })
  it('on a phone shows up to three, else two plus "+N"', () => {
    expect([0, 1, 3, 4, 9].map((n) => weekVisible(n, true))).toEqual([0, 1, 3, 2, 2])
  })
  it('gives every row a fixed height through a clipped box', () => {
    const css = readFileSync(resolve(__dirname, 'WeekGrid.vue'), 'utf8')
    expect(css).toMatch(/\.cellbox \{[^}]*height: calc\(var\(--wg-row-h\)[^}]*overflow: hidden/)
  })
})
