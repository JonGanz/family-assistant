import { describe, expect, it } from 'vitest'
import { invalidates } from './useLiveStream'
import { rangeFor, shiftAnchor } from './useRange'
import { useUnits } from './useUnits'
import { ref } from 'vue'

describe('useRange', () => {
  it('week starts Monday, month spans calendar month', () => {
    expect(rangeFor('week', '2026-10-24')).toEqual({ from: '2026-10-19', to: '2026-10-26' })
    expect(rangeFor('month', '2026-12-10')).toEqual({ from: '2026-12-01', to: '2027-01-01' })
    expect(rangeFor('day', '2026-10-24')).toEqual({ from: '2026-10-24', to: '2026-10-25' })
  })
  it('shifts', () => {
    expect(shiftAnchor('month', '2026-01-15', -1)).toBe('2025-12-01')
    expect(shiftAnchor('week', '2026-10-24', 1)).toBe('2026-10-31')
    expect(shiftAnchor('day', '2026-10-31', 1)).toBe('2026-11-01')
  })
})

describe('invalidates', () => {
  const held = { kind: 'calendar', from: '2026-10-19', to: '2026-10-26' } as const
  it('calendar only when overlapping', () => {
    expect(invalidates('calendar.updated', { from: '2026-10-25', to: '2026-10-27' }, held)).toBe(
      true,
    )
    expect(invalidates('calendar.updated', { from: '2026-10-26', to: '2026-10-27' }, held)).toBe(
      false,
    )
    expect(invalidates('calendar.updated', undefined, held)).toBe(true)
    expect(
      invalidates(
        'calendar.updated',
        { from: '2026-10-25', to: '2026-10-27' },
        { kind: 'weather' },
      ),
    ).toBe(false)
  })
  it('resync hits everything; others are kind-scoped', () => {
    expect(invalidates('resync', undefined, { kind: 'member' })).toBe(true)
    expect(invalidates('weather.updated', undefined, { kind: 'weather' })).toBe(true)
    expect(invalidates('weather.updated', undefined, held)).toBe(false)
    expect(invalidates('device.updated', { id: 'a' }, { kind: 'device', id: 'b' })).toBe(false)
    expect(invalidates('device.updated', { id: 'a' }, { kind: 'device', id: 'a' })).toBe(true)
  })
})

describe('useUnits', () => {
  it('reacts to the units ref', () => {
    const u = ref<'metric' | 'imperial'>('metric')
    const { format } = useUnits(u)
    expect(format(0, 'temp')).toBe('0°C')
    u.value = 'imperial'
    expect(format(0, 'temp')).toBe('32°F')
  })
})
