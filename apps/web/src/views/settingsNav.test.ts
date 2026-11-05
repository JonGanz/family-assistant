import { describe, expect, it } from 'vitest'
import { visibleGroups } from './settingsNav'

describe('settings map', () => {
  it('shows a child only the Personal group', () => {
    const g = visibleGroups(false)
    expect(g.map((x) => x.title)).toEqual(['Personal'])
    expect(g[0]!.pages.map((p) => p.to)).toContain('/settings/notifications')
  })

  it('shows an adult every group, with Members and System', () => {
    const g = visibleGroups(true)
    expect(g.map((x) => x.title)).toEqual(['Personal', 'Household', 'Sources and rules', 'System'])
    const to = g.flatMap((x) => x.pages.map((p) => p.to))
    expect(to).toContain('/settings/members')
    expect(to).toContain('/settings/system')
  })
})

describe('settings map entries', () => {
  it('give every page an icon and a one-line summary', () => {
    for (const p of visibleGroups(true).flatMap((g) => g.pages)) {
      expect(p.icon, p.to).toBeTruthy()
      expect(p.summary, p.to).toBe(p.blurb)
    }
  })

  it('lists the AI page for adults only', () => {
    const adult = visibleGroups(true).flatMap((g) => g.pages.map((p) => p.to))
    const child = visibleGroups(false).flatMap((g) => g.pages.map((p) => p.to))
    expect(adult).toContain('/settings/ai')
    expect(child).not.toContain('/settings/ai')
  })
})
