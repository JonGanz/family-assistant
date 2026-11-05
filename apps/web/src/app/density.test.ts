import { beforeEach, describe, expect, it } from 'vitest'
import { applyDensity, getDensity, setDensity } from './density'

beforeEach(() => {
  localStorage.clear()
  document.documentElement.removeAttribute('data-density')
})

describe('density', () => {
  it('defaults to comfortable', () => {
    expect(getDensity()).toBe('comfortable')
    applyDensity()
    expect(document.documentElement.dataset.density).toBe('comfortable')
  })

  it('stores and applies compact per device', () => {
    setDensity('compact')
    expect(localStorage.getItem('fa.density')).toBe('compact')
    expect(getDensity()).toBe('compact')
    expect(document.documentElement.dataset.density).toBe('compact')
  })

  it('ignores a junk stored value', () => {
    localStorage.setItem('fa.density', 'huge')
    expect(getDensity()).toBe('comfortable')
  })
})
