import { beforeEach, describe, expect, it } from 'vitest'
import { captureDeviceToken, getDeviceToken } from './deviceToken'

describe('captureDeviceToken', () => {
  beforeEach(() => {
    localStorage.clear()
    history.replaceState(null, '', '/dashboard')
  })

  it('moves the token from the query string into localStorage and strips it from the URL', () => {
    history.replaceState(null, '', '/dashboard?device=abc123&x=1')
    expect(captureDeviceToken()).toBe('abc123')
    expect(localStorage.getItem('fa.deviceToken')).toBe('abc123')
    expect(location.search).toBe('?x=1')
    expect(location.pathname).toBe('/dashboard')
    expect(getDeviceToken()).toBe('abc123')
  })

  it('falls back to the stored token when there is no hash', () => {
    localStorage.setItem('fa.deviceToken', 'stored')
    expect(captureDeviceToken()).toBe('stored')
  })

  it('survives unavailable storage', () => {
    history.replaceState(null, '', '/dashboard?device=zzz')
    const orig = Storage.prototype.setItem
    Storage.prototype.setItem = () => {
      throw new Error('denied')
    }
    try {
      expect(captureDeviceToken()).toBe('zzz')
      expect(location.hash).toBe('')
    } finally {
      Storage.prototype.setItem = orig
    }
  })
})
