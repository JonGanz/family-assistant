import { describe, expect, it } from 'vitest'
import { hueVar, isHue, textOn } from './hues'

describe('hues', () => {
  it('maps a hue to its variable and unset to blue', () => {
    expect(hueVar('red')).toBe('var(--hue-red)')
    expect(hueVar(null)).toBe('var(--hue-blue)')
    expect(hueVar(undefined)).toBe('var(--hue-blue)')
    expect(hueVar('#ff0000')).toBe('var(--hue-blue)')
  })
  it('recognises hue names', () => {
    expect(isHue('pink')).toBe(true)
    expect(isHue('teal')).toBe(false)
  })
  it('uses the theme on variables and computes for hex', () => {
    expect(textOn('var(--hue-red)')).toBe('var(--color-bg)')
    expect(textOn('#000000')).not.toBe('var(--color-bg)')
  })
})
