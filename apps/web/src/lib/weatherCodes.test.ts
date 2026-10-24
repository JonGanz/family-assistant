import { describe, expect, it } from 'vitest'
import { weatherInfo, WEATHER_CODES } from './weatherCodes'

describe('weatherCodes', () => {
  it('covers every Open-Meteo code', () => {
    const codes = [
      0, 1, 2, 3, 45, 48, 51, 53, 55, 56, 57, 61, 63, 65, 66, 67, 71, 73, 75, 77, 80, 81, 82, 85,
      86, 95, 96, 99,
    ]
    for (const c of codes) expect(WEATHER_CODES[c], String(c)).toBeDefined()
  })
  it('night icon and unknown fallback', () => {
    expect(weatherInfo(0, true).icon).toBe('clear-night')
    expect(weatherInfo(0, false).icon).toBe('clear')
    expect(weatherInfo(61, true).icon).toBe('rain')
    expect(weatherInfo(1234).label).toBe('Unknown')
  })
})
