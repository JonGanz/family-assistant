import { afterEach, describe, expect, it, vi } from 'vitest'
import { goBack } from './goBack'

describe('goBack', () => {
  afterEach(() => window.history.replaceState(null, ''))

  it('goes back when the router has history', () => {
    window.history.replaceState({ back: '/food/plan' }, '')
    const router = { back: vi.fn(), push: vi.fn() }
    goBack(router)
    expect(router.back).toHaveBeenCalled()
    expect(router.push).not.toHaveBeenCalled()
  })

  it('pushes the fallback without history', () => {
    window.history.replaceState({ back: null }, '')
    const router = { back: vi.fn(), push: vi.fn() }
    goBack(router)
    expect(router.push).toHaveBeenCalledWith('/food/recipes')
    expect(router.back).not.toHaveBeenCalled()
  })
})
