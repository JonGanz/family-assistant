import { describe, expect, it } from 'vitest'
import { eventColors, stripeStyle } from './eventColors'
import type { MemberLike } from './types'

const members: MemberLike[] = [
  { id: 'a', display_name: 'A', color: '#111111' },
  { id: 'b', display_name: 'B', color: '#222222' },
]

describe('eventColors', () => {
  it('uses the single colour for an ordinary event', () => {
    expect(eventColors({ color: '#abcdef', members: ['a'] }, members, '#000000')).toEqual([
      '#abcdef',
    ])
  })

  it('falls back to the first member, then the default', () => {
    expect(eventColors({ members: ['b'] }, members, '#000000')).toEqual(['#222222'])
    expect(eventColors({ members: [] }, members, '#000000')).toEqual(['#000000'])
  })

  it('shows every calendar colour of a merged event', () => {
    const ev = {
      color: '#aa0000',
      colors: ['#aa0000', '#00bb00'],
      merged: true,
      members: ['a', 'b'],
    }
    expect(eventColors(ev, members, '#000000')).toEqual(['#aa0000', '#00bb00'])
  })

  it('uses the members colours for a merged event whose calendars have none', () => {
    const ev = { colors: [], merged: true, members: ['a', 'b'] }
    expect(eventColors(ev, members, '#000000')).toEqual(['#111111', '#222222'])
  })
})

describe('stripeStyle', () => {
  it('is a plain border colour for one colour', () => {
    expect(stripeStyle({ color: '#111111', colors: ['#111111'] })).toEqual({
      borderLeftColor: '#111111',
    })
  })

  it('is equal bands for several', () => {
    const s = stripeStyle({ color: '#111111', colors: ['#111111', '#222222'] })
    expect(s.borderImage).toBe('linear-gradient(to bottom, #111111 0% 50%, #222222 50% 100%) 1')
  })
})
