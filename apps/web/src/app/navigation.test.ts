import { describe, expect, it } from 'vitest'
import { buildNavigation } from './navigation'

const ids = (l: { id: string }[]) => l.map((d) => d.id)

describe('navigation', () => {
  it('hides Inbox, Mail and Bills from a child (D-063)', () => {
    const n = buildNavigation(false, 5)
    expect(ids(n.all)).toEqual(['today', 'calendar', 'tasks', 'food', 'routines', 'settings'])
    expect(ids(n.all)).not.toContain('inbox')
  })

  it('gives an adult everything and the inbox badge', () => {
    const n = buildNavigation(true, 3)
    expect(ids(n.all)).toEqual([
      'today',
      'calendar',
      'tasks',
      'food',
      'inbox',
      'routines',
      'mail',
      'bills',
      'settings',
    ])
    expect(n.all.find((d) => d.id === 'inbox')?.badge).toBe(3)
    expect(buildNavigation(true, 0).all.find((d) => d.id === 'inbox')?.badge).toBeUndefined()
  })

  it('lists the Food tabs under a Food group in the drawer, Templates for adults only', () => {
    const adult = buildNavigation(true, 0).drawer.filter((d) => d.group === 'Food')
    expect(ids(adult)).toEqual(['food-plan', 'food-list', 'food-recipes', 'food-templates'])
    expect(buildNavigation(true, 0).drawer.some((d) => d.id === 'food')).toBe(false)
    const child = buildNavigation(false, 0).drawer.filter((d) => d.group === 'Food')
    expect(ids(child)).toEqual(['food-plan', 'food-list', 'food-recipes'])
  })

  it('splits the primary destinations from the More list', () => {
    const adult = buildNavigation(true, 0)
    expect(ids(adult.primary)).toEqual(['today', 'calendar', 'tasks', 'food', 'inbox'])
    expect(ids(adult.more)).toEqual(['routines', 'mail', 'bills', 'settings'])
    const child = buildNavigation(false, 0)
    expect(ids(child.primary)).toEqual(['today', 'calendar', 'tasks', 'food'])
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
