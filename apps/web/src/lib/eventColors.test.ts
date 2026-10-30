import { describe, expect, it } from 'vitest'
import { eventColors, stripeStyle } from './eventColors'
import type { MemberLike } from './types'

const members: MemberLike[] = [
  { id: 'a', display_name: 'A', color: '#111111' },
  { id: 'b', display_name: 'B', color: '#222222' },
]

describe('eventColors', () => {
  it('uses the colour of the member', () => {
    expect(eventColors({ members: ['b'] }, members, '#000000')).toEqual(['#222222'])
  })

  it('shows every member colour of an event for several members', () => {
    expect(eventColors({ members: ['b', 'a'] }, members, '#000000')).toEqual(['#111111', '#222222'])
  })

  it('falls back to the default for an event with no member', () => {
    expect(eventColors({ members: [] }, members, '#000000')).toEqual(['#000000'])
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

describe('eventColors with hue variables', () => {
  const hues: MemberLike[] = [
    { id: 'a', display_name: 'A', color: 'var(--hue-green)' },
    { id: 'b', display_name: 'B', color: 'var(--hue-red)' },
  ]
  it('uses the member hues, and the fallback for an event with no member', () => {
    expect(eventColors({ members: ['a', 'b'] }, hues, 'var(--hue-blue)')).toEqual([
      'var(--hue-green)',
      'var(--hue-red)',
    ])
    expect(eventColors({ members: [] }, hues, 'var(--hue-blue)')).toEqual(['var(--hue-blue)'])
  })
})
