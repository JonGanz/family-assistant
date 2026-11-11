import { describe, expect, it } from 'vitest'
import { mapLink } from './mapLink'

describe('mapLink', () => {
  it('is null without a location', () => {
    expect(mapLink('  ', 'x')).toBeNull()
  })
  it('uses the native app on Apple and Android', () => {
    expect(mapLink('1 Main St', 'Mozilla (iPhone; CPU iPhone OS 17)')?.href).toBe(
      'maps:?q=1%20Main%20St',
    )
    expect(mapLink('1 Main St', 'Mozilla (Linux; Android 14)')?.href).toBe(
      'geo:0,0?q=1%20Main%20St',
    )
  })
  it('falls back to Google Maps in a new tab', () => {
    expect(mapLink('Park', 'Mozilla (X11; Linux x86_64)')).toEqual({
      href: 'https://www.google.com/maps/search/?api=1&query=Park',
      external: true,
    })
  })
})
