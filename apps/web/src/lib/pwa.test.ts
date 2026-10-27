import { afterEach, describe, expect, it, vi } from 'vitest'
import { registerServiceWorker } from './pwa'

afterEach(() => vi.unstubAllGlobals())

describe('registerServiceWorker', () => {
  it('does nothing in an insecure context', async () => {
    const register = vi.fn()
    vi.stubGlobal('navigator', { serviceWorker: { register } })
    expect(await registerServiceWorker(false)).toBe(false)
    expect(register).not.toHaveBeenCalled()
  })

  it('registers in a secure context when the browser supports it', async () => {
    const register = vi.fn().mockResolvedValue({})
    vi.stubGlobal('navigator', { serviceWorker: { register } })
    expect(await registerServiceWorker(true)).toBe(true)
    expect(register).toHaveBeenCalledWith('/sw.js')
  })

  it('is false when the browser has no service workers', async () => {
    vi.stubGlobal('navigator', {})
    expect(await registerServiceWorker(true)).toBe(false)
  })
})
