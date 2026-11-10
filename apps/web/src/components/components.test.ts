import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import Avatar from './Avatar.vue'
import DateRangeNav from './DateRangeNav.vue'
import EventCard from './EventCard.vue'
import MemberChip from './MemberChip.vue'
import WeatherIcon from './WeatherIcon.vue'
import type { EventLike } from '../lib/types'

const event: EventLike = {
  title: 'Piano',
  location: 'Hall',
  start_at: '2026-03-08T15:00:00Z',
  end_at: '2026-03-08T16:00:00Z',
  all_day: false,
  members: ['m1'],
  color: '#336699',
  visibility: 'full',
  source_url: 'https://example.com/e',
}
const members = [{ id: 'm1', display_name: 'Alex Example', color: '#ffeb3b' }]

describe('components', () => {
  it('MemberChip computes text colour', () => {
    const w = mount(MemberChip, { props: { name: 'Alex', color: '#ffeb3b' } })
    expect(w.text()).toBe('Alex')
    expect(w.attributes('style')).toContain('color: rgb(0, 0, 0)')
  })
  it('MemberChip and Avatar keep the kiosk look unless asked for the app variant', () => {
    expect(mount(MemberChip, { props: { name: 'A', color: 'red' } }).classes()).not.toContain(
      'member-chip--app',
    )
    expect(
      mount(MemberChip, { props: { name: 'A', color: 'red', variant: 'app' } }).classes(),
    ).toContain('member-chip--app')
    expect(mount(Avatar, { props: { name: 'A', color: 'red' } }).classes()).not.toContain(
      'avatar--app',
    )
  })
  it('EventCard emits select when selectable', async () => {
    const w = mount(EventCard, {
      props: { event, tz: 'UTC', members, selectable: true },
    })
    expect(w.classes()).not.toContain('square')
    await w.trigger('click')
    expect(w.emitted('select')).toHaveLength(1)
  })
  it('Avatar falls back to initials and uses image when given', () => {
    expect(mount(Avatar, { props: { name: 'Alex Example', color: '#336699' } }).text()).toBe('AE')
    const w = mount(Avatar, {
      props: { name: 'Alex', color: '#336699', src: 'https://example.com/a.png' },
    })
    expect(w.find('img').attributes('src')).toBe('https://example.com/a.png')
  })
  it('EventCard shows details, chips and link', () => {
    const w = mount(EventCard, {
      props: { event, tz: 'America/New_York', members, locale: 'en-US' },
    })
    expect(w.text()).toContain('Piano')
    expect(w.text()).toContain('Hall')
    expect(w.text()).toContain('Alex Example')
    expect(w.get('a.from').attributes('href')).toBe('https://example.com/e')
    expect(w.classes()).not.toContain('busy')
  })
  it('EventCard hides details when busy', () => {
    const w = mount(EventCard, {
      props: { event: { ...event, visibility: 'busy' }, tz: 'UTC', members },
    })
    expect(w.classes()).toContain('busy')
    expect(w.text()).toContain('Busy')
    expect(w.text()).not.toContain('Piano')
    expect(w.text()).not.toContain('Hall')
  })
  it('WeatherIcon labels by code', () => {
    const w = mount(WeatherIcon, { props: { code: 95 } })
    expect(w.attributes('aria-label')).toBe('Thunderstorm')
    expect(w.find('svg').exists()).toBe(true)
  })
  it('DateRangeNav emits', async () => {
    const w = mount(DateRangeNav, { props: { label: 'Mar 2026', mode: 'week' } })
    await w.get('[aria-label=Next]').trigger('click')
    await w.get('[aria-label=Previous]').trigger('click')
    await w
      .findAll('[role=radio]')
      .find((b) => b.text() === 'Month')!
      .trigger('click')
    expect(w.emitted('next')).toHaveLength(1)
    expect(w.emitted('prev')).toHaveLength(1)
    expect(w.emitted('update:mode')).toEqual([['month']])
  })
})
