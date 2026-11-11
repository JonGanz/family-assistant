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
  it('has the same structure for 1, 2 and 6 colours', () => {
    const mk = (n: number) =>
      stripeStyle({ color: '#000', colors: Array.from({ length: n }, (_, i) => `#00000${i}`) })
    const keys = Object.keys(mk(1)).sort()
    for (const n of [2, 6]) expect(Object.keys(mk(n)).sort()).toEqual(keys)
    expect(mk(1).borderLeftColor).toBe('transparent')
    expect(mk(1).backgroundImage).toBe(
      'linear-gradient(to bottom, #000000 0% 100%), var(--ev-bg, none)',
    )
    expect(mk(6).backgroundImage).toContain('#000005 83.33333333333334% 100%')
  })

  it('falls back to color when colors is empty', () => {
    expect(stripeStyle({ color: '#123456', colors: [] }).backgroundImage).toContain(
      '#123456 0% 100%',
    )
  })

  it('is equal bands for several, painted under the border so corners stay round', () => {
    const s = stripeStyle({ color: '#111111', colors: ['#111111', '#222222'] })
    expect(s.borderImage).toBeUndefined()
    expect(s.borderLeftColor).toBe('transparent')
    expect(s.backgroundImage).toBe(
      'linear-gradient(to bottom, #111111 0% 50%, #222222 50% 100%), var(--ev-bg, none)',
    )
    expect(s.backgroundSize).toBe('var(--ev-stripe-w, 4px) 100%, auto')
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
