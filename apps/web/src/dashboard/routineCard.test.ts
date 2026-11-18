import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import DashboardCanvas from './DashboardCanvas.vue'
import { sampleRoutines } from './preview'
import type { DashboardData, DashboardSettings } from './types'

const NOW = new Date('2026-10-24T10:00:00Z')

function mk(allDone: boolean) {
  const members = [
    {
      id: 'm1',
      display_name: 'Alex',
      role: 'child',
      avatar: null,
      avatar_url: null,
      color: '#36c',
    },
  ]
  const run = sampleRoutines(members as never, NOW, 'UTC')[0]!
  if (allDone) run.steps.forEach((s) => (s.done_at = NOW.toISOString()))
  const settings = {
    theme: { family: 'catppuccin', mode: 'light' },
    agenda_days: 2,
    members: 'all',
    panels: [{ kind: 'clock' }, { kind: 'routine' }],
  } as unknown as DashboardSettings
  const data = {
    household: { name: 'H', tz: 'UTC', units: 'metric', location: null },
    members,
    events: [],
    advisories: [],
    weather: null,
    routines: [run],
    settings,
  } as unknown as DashboardData
  return mount(DashboardCanvas, { props: { data, settings, now: NOW } })
}

describe('completed routine highlight', () => {
  it('marks the outer card, not an inner box', async () => {
    const w = mk(true)
    await w.vm.$nextTick()
    expect(w.find('.cell.k-routine').classes()).toContain('complete')
    expect(w.find('.cell.k-routine').attributes('data-complete')).toBe('true')
    expect(w.find('[data-run]').classes()).not.toContain('complete')
  })
  it('does not mark an unfinished routine', async () => {
    const w = mk(false)
    await w.vm.$nextTick()
    expect(w.find('.cell.k-routine').classes()).not.toContain('complete')
  })
})
