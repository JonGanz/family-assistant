import { afterEach, describe, expect, it, vi } from 'vitest'
import { getNavCollapsed, setNavCollapsed } from './navPrefs'

afterEach(() => {
  vi.restoreAllMocks()
  localStorage.clear()
})

describe('navPrefs', () => {
  it('round-trips through localStorage', () => {
    expect(getNavCollapsed()).toBe(false)
    setNavCollapsed(true)
    expect(localStorage.getItem('fa.nav.collapsed')).toBe('1')
    expect(getNavCollapsed()).toBe(true)
    setNavCollapsed(false)
    expect(localStorage.getItem('fa.nav.collapsed')).toBeNull()
    expect(getNavCollapsed()).toBe(false)
  })

  it('survives a throwing localStorage', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    expect(getNavCollapsed()).toBe(false)
    expect(() => setNavCollapsed(true)).not.toThrow()
  })
})
