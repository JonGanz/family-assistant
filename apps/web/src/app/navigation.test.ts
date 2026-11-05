import { describe, expect, it } from 'vitest'
import { buildNavigation } from './navigation'

const ids = (l: { id: string }[]) => l.map((d) => d.id)

describe('navigation', () => {
  it('hides Inbox, Mail and Bills from a child (D-063)', () => {
    const n = buildNavigation(false, 5)
    expect(ids(n.all)).toEqual(['today', 'calendar', 'tasks', 'routines', 'settings'])
    expect(ids(n.all)).not.toContain('inbox')
  })

  it('gives an adult everything and the inbox badge', () => {
    const n = buildNavigation(true, 3)
    expect(ids(n.all)).toEqual([
      'today',
      'calendar',
      'tasks',
      'inbox',
      'routines',
      'mail',
      'bills',
      'settings',
    ])
    expect(n.all.find((d) => d.id === 'inbox')?.badge).toBe(3)
    expect(buildNavigation(true, 0).all.find((d) => d.id === 'inbox')?.badge).toBeUndefined()
  })

  it('splits the bar (at most 4 plus More) from the More list', () => {
    const adult = buildNavigation(true, 0)
    expect(ids(adult.primary)).toEqual(['today', 'calendar', 'tasks', 'inbox'])
    expect(ids(adult.more)).toEqual(['routines', 'mail', 'bills', 'settings'])
    const child = buildNavigation(false, 0)
    expect(ids(child.primary)).toEqual(['today', 'calendar', 'tasks'])
    expect(ids(child.more)).toEqual(['routines', 'settings'])
  })
})

describe('nav glyphs', () => {
  it('shows the filled glyph when active and the outline pair otherwise', async () => {
    const { navGlyph } = await import('../ui')
    expect(navGlyph('tasks', true)).toBe('tasks')
    expect(navGlyph('tasks', false)).toBe('tasks_outline')
    expect(navGlyph('back', false)).toBe('back')
  })
})
