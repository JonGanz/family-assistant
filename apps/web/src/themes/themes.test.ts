import { afterEach, describe, expect, it, vi } from 'vitest'
import { contrastRatio } from '../lib/contrast'
import {
  DEFAULT_CHOICE,
  FAMILIES,
  HUES,
  applyTheme,
  loadDeviceChoice,
  resolveVariant,
  saveDeviceChoice,
} from './index'

const HEX = /^#[0-9a-f]{6}$/i
const SLUG = /^[a-z0-9][a-z0-9-]{0,31}$/
const NEUTRALS = ['bg', 'surface', 'surface2', 'border', 'muted', 'text', 'onAccent'] as const

describe('catalogue structure', () => {
  it('has unique slug ids and at least one variant per family', () => {
    const ids = FAMILIES.map((f) => f.id)
    expect(new Set(ids).size).toBe(ids.length)
    for (const f of FAMILIES) {
      expect(f.id).toMatch(SLUG)
      expect(f.source).toMatch(/^https?:\/\//)
      expect(f.license).not.toBe('')
      expect(f.variants.length).toBeGreaterThan(0)
      const vids = f.variants.map((v) => v.id)
      expect(new Set(vids).size).toBe(vids.length)
      for (const v of f.variants) expect(v.id).toMatch(SLUG)
    }
  })

  it('fills every token with #rrggbb', () => {
    for (const f of FAMILIES)
      for (const v of f.variants) {
        for (const k of NEUTRALS) expect(v.neutrals[k], `${f.id}/${v.id} ${k}`).toMatch(HEX)
        expect(Object.keys(v.hues).sort()).toEqual([...HUES].sort())
        for (const h of HUES) expect(v.hues[h], `${f.id}/${v.id} ${h}`).toMatch(HEX)
      }
  })
})

describe('contrast rules', () => {
  for (const f of FAMILIES)
    for (const v of f.variants)
      describe(`${f.id}/${v.id}`, () => {
        const n = v.neutrals
        it('text and muted reach 4.5:1 on bg, surface and surface2', () => {
          for (const fg of [n.text, n.muted])
            for (const bg of [n.bg, n.surface, n.surface2])
              expect(contrastRatio(fg, bg), `${fg} on ${bg}`).toBeGreaterThanOrEqual(4.5)
        })
        it('each hue reaches 3:1 on bg', () => {
          for (const h of HUES) expect(contrastRatio(v.hues[h], n.bg), h).toBeGreaterThanOrEqual(3)
        })
        it('onAccent reaches 4.5:1 on blue', () => {
          expect(contrastRatio(n.onAccent, v.hues.blue)).toBeGreaterThanOrEqual(4.5)
        })
      })
})

describe('resolveVariant', () => {
  it('follows the mode for a two-mode family', () => {
    expect(
      resolveVariant({ family: 'catppuccin', mode: 'dark' }, { prefersDark: false }).mode,
    ).toBe('dark')
    expect(
      resolveVariant({ family: 'catppuccin', mode: 'light' }, { prefersDark: true }).mode,
    ).toBe('light')
  })
  it('auto uses night when given, else prefersDark', () => {
    const c = { family: 'catppuccin', mode: 'auto' as const }
    expect(resolveVariant(c, { prefersDark: false, night: true }).mode).toBe('dark')
    expect(resolveVariant(c, { prefersDark: true, night: false }).mode).toBe('light')
    expect(resolveVariant(c, { prefersDark: true }).mode).toBe('dark')
  })
  it('falls back to the default family when unknown', () => {
    expect(resolveVariant({ family: 'nope', mode: 'light' }, { prefersDark: false }).id).toBe(
      'latte',
    )
  })
  it('honours a variant id, ignoring an unknown one', () => {
    const c = { family: 'catppuccin', mode: 'dark' as const }
    expect(resolveVariant({ ...c, variant: 'mocha' }, { prefersDark: false }).id).toBe('mocha')
    expect(resolveVariant({ ...c, variant: 'zzz' }, { prefersDark: false }).id).toBe('mocha')
  })
  it('a single-mode family ignores the mode', () => {
    const fam = FAMILIES[0]!
    const saved = fam.variants
    fam.variants = saved.filter((v) => v.mode === 'dark')
    try {
      expect(resolveVariant({ family: fam.id, mode: 'light' }, { prefersDark: false }).mode).toBe(
        'dark',
      )
    } finally {
      fam.variants = saved
    }
  })
})

describe('device choice storage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    localStorage.clear()
  })
  it('round-trips', () => {
    saveDeviceChoice({ family: 'catppuccin', variant: 'mocha', mode: 'dark' })
    expect(loadDeviceChoice()).toEqual({ family: 'catppuccin', variant: 'mocha', mode: 'dark' })
  })
  it('falls back on missing or bad JSON', () => {
    expect(loadDeviceChoice()).toEqual(DEFAULT_CHOICE)
    localStorage.setItem('fa.theme', '{nope')
    expect(loadDeviceChoice()).toEqual(DEFAULT_CHOICE)
    localStorage.setItem('fa.theme', '{"mode":"dark"}')
    expect(loadDeviceChoice()).toEqual(DEFAULT_CHOICE)
  })
  it('survives unavailable storage', () => {
    const broken = {
      getItem: () => {
        throw new Error('denied')
      },
      setItem: () => {
        throw new Error('denied')
      },
    }
    vi.stubGlobal('localStorage', broken)
    expect(loadDeviceChoice()).toEqual(DEFAULT_CHOICE)
    expect(() => saveDeviceChoice(DEFAULT_CHOICE)).not.toThrow()
  })
})

describe('applyTheme', () => {
  it('writes variables and data-theme-mode', () => {
    const el = document.createElement('div')
    const v = resolveVariant({ family: 'catppuccin', mode: 'dark' }, { prefersDark: false })
    applyTheme(v, el)
    expect(el.getAttribute('data-theme-mode')).toBe('dark')
    expect(el.style.getPropertyValue('--color-bg')).toBe(v.neutrals.bg)
    expect(el.style.getPropertyValue('--color-surface-2')).toBe(v.neutrals.surface2)
    expect(el.style.getPropertyValue('--hue-pink')).toBe(v.hues.pink)
    expect(el.style.getPropertyValue('--color-accent')).toBe(v.hues.blue)
    expect(el.style.getPropertyValue('--color-danger')).toBe(v.hues.red)
  })
})
