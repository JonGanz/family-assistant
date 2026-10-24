import { describe, expect, it } from 'vitest'
import { convert, formatQuantity } from './units'

describe('units', () => {
  it('metric is identity', () => {
    expect(convert(21, 'temp', 'metric')).toBe(21)
  })
  it('converts to imperial', () => {
    expect(convert(100, 'temp', 'imperial')).toBe(212)
    expect(convert(25.4, 'precip', 'imperial')).toBeCloseTo(1)
    expect(convert(2.54, 'length', 'imperial')).toBeCloseTo(1)
    expect(convert(1.609344, 'speed', 'imperial')).toBeCloseTo(1)
  })
  it('formats with suffix', () => {
    expect(formatQuantity(20, 'temp', 'imperial')).toBe('68°F')
    expect(formatQuantity(20, 'temp', 'metric')).toBe('20°C')
    expect(formatQuantity(10, 'speed', 'imperial')).toBe('6 mph')
    expect(formatQuantity(5, 'precip', 'metric')).toBe('5 mm')
    expect(formatQuantity(5, 'precip', 'imperial')).toBe('0.2 in')
  })
})
