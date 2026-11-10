import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { stripeStyle } from '../lib/eventColors'

const read = (f: string) => readFileSync(new URL(f, import.meta.url), 'utf8')

describe('event items look the same for one or several members', () => {
  it('multi-colour stripes do not use border-image (it drops the radius)', () => {
    const s = stripeStyle({ color: '#111', colors: ['#111', '#222'] })
    expect(s.borderImage).toBeUndefined()
    expect(s.borderLeftColor).toBe('transparent')
  })
  it.each(['./DayCell.vue', './panels/AgendaPanel.vue'])(
    '%s rounds every event and sets the stripe width',
    (f) => {
      const css = read(f)
      const rule = /(?:li)?\.event\s*\{[^}]*--ev-stripe-w: 6px[^}]*\}/.exec(css)?.[0] ?? ''
      expect(rule).toContain('border-left: 6px')
      expect(rule).toContain('border-radius: var(--ui-radius-sm)')
      expect(css).not.toMatch(/border-radius:\s*0/)
    },
  )
})
