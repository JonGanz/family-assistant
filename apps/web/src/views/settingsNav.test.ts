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
