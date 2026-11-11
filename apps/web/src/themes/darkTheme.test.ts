import { describe, expect, it } from 'vitest'
import { resolveVariant } from '.'
import { normalizeTheme } from '../dashboard/layout'
import { withDarkFamily, withFamily } from '../dashboard/editor'
import type { ThemeChoice } from './types'

const choice: ThemeChoice = {
  family: 'catppuccin',
  mode: 'auto',
  dark: { family: 'nord' },
}

describe('night-time theme', () => {
  it('uses the daytime family by day and the night family when dark', () => {
    const day = resolveVariant(choice, { prefersDark: false })
    expect(day.mode).toBe('light')
    const night = resolveVariant(choice, { prefersDark: false, night: true })
    expect(night.mode).toBe('dark')
    expect(night.id).toContain('nord')
  })
  it('follows system dark under auto and an explicit dark mode', () => {
    expect(resolveVariant(choice, { prefersDark: true }).id).toContain('nord')
    expect(resolveVariant({ ...choice, mode: 'dark' }, { prefersDark: false }).id).toContain('nord')
    expect(resolveVariant({ ...choice, mode: 'light' }, { prefersDark: true }).mode).toBe('light')
  })
  it('without a night theme behaves as before', () => {
    const c: ThemeChoice = { family: 'catppuccin', mode: 'dark' }
    expect(resolveVariant(c, { prefersDark: false }).id).toContain('mocha')
  })
  it('round-trips through normalizeTheme and the editor helpers', () => {
    expect(normalizeTheme(choice)).toEqual(choice)
    expect(normalizeTheme({ family: 'nord', mode: 'auto' }).dark).toBeUndefined()
    expect(withFamily(choice, 'dracula').dark).toEqual(choice.dark)
    expect(withDarkFamily(choice, null).dark).toBeUndefined()
  })
})
