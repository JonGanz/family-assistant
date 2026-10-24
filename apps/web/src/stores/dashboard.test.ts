import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'
import type { DashboardData } from '../dashboard/types'

vi.mock('../api/client', () => ({
  api: { me: vi.fn(), dashboard: vi.fn(), dismissAdvisory: vi.fn() },
}))
import { api } from '../api/client'
import { CACHE_KEY, daysToFetch, readCache, useDashboardStore } from './dashboard'

const dash = vi.mocked(api.dashboard)

function snap(over: Partial<DashboardData['settings']> = {}): DashboardData {
  return {
    household: { name: 'Example Household', tz: 'UTC', units: 'metric', location: null },
    members: [],
    events: [],
    advisories: [],
    weather: null,
    settings: {
      orientation: 'landscape',
      theme: 'auto',
      agenda_days: 3,
      members: 'all',
      panels: [{ kind: 'clock' }],
      ...over,
    },
  } as unknown as DashboardData
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date('2026-10-24T12:00:00Z'))
  setActivePinia(createPinia())
  localStorage.clear()
  dash.mockReset()
  vi.mocked(api.me).mockReset()
})
afterEach(() => vi.useRealTimers())

async function loaded() {
  dash.mockResolvedValue(snap())
  const s = useDashboardStore()
  await s.refresh()
  s.deviceId = 'dev-1'
  dash.mockClear()
  return s
}

describe('dashboard store', () => {
  it('fetches days from settings and caches the full snapshot with savedAt', async () => {
    const s = await loaded()
    expect(s.householdName).toBe('Example Household')
    const c = readCache()
    expect(c?.snapshot.household.name).toBe('Example Household')
    expect(c?.savedAt).toBe(Date.now())
    expect(s.stale).toBe(false)
  })

  it('falls back to the cache with the stale badge when the API fails', async () => {
    await loaded()
    setActivePinia(createPinia())
    dash.mockRejectedValue(new Error('offline'))
    const s = useDashboardStore()
    await s.refresh()
    expect(s.stale).toBe(true)
    expect(s.showStaleBadge).toBe(true)
    expect(s.householdName).toBe('Example Household')
    expect(s.savedAt).toBeTypeOf('number')
  })

  it('ignores a cache from an older format', () => {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ householdName: 'x', members: [], savedAt: 1 }))
    expect(readCache()).toBeNull()
  })

  it('shows the badge while the stream is not open', async () => {
    const s = await loaded()
    s.connection = 'open'
    expect(s.showStaleBadge).toBe(false)
    s.connection = 'reconnecting'
    expect(s.showStaleBadge).toBe(true)
  })

  it('calendar.updated refetches only when its range overlaps the shown window', async () => {
    const s = await loaded()
    // window: 2026-10-24 .. 2026-10-27 (3 days, UTC)
    expect(
      s.handleEvent('calendar.updated', {
        from: '2026-11-10T00:00:00Z',
        to: '2026-11-20T00:00:00Z',
      }),
    ).toBe(false)
    expect(
      s.handleEvent('calendar.updated', {
        from: '2026-10-01T00:00:00Z',
        to: '2026-10-24T00:00:00Z',
      }),
    ).toBe(false)
    expect(dash).not.toHaveBeenCalled()
    expect(
      s.handleEvent('calendar.updated', {
        from: '2026-10-25T00:00:00Z',
        to: '2026-10-26T00:00:00Z',
      }),
    ).toBe(true)
    await s.refresh()
    expect(dash).toHaveBeenCalledTimes(1)
  })

  it.each(['weather.updated', 'advisory.updated', 'member.updated', 'household.updated', 'resync'])(
    '%s refetches',
    async (kind) => {
      const s = await loaded()
      expect(s.handleEvent(kind, undefined)).toBe(true)
      await s.refresh()
      expect(dash).toHaveBeenCalled()
    },
  )

  it('device.updated refetches for this device only', async () => {
    const s = await loaded()
    expect(s.handleEvent('device.updated', { id: 'other' })).toBe(false)
    expect(dash).not.toHaveBeenCalled()
    expect(s.handleEvent('device.updated', { id: 'dev-1' })).toBe(true)
  })

  it('ignores kinds it does not show', async () => {
    const s = await loaded()
    expect(s.handleEvent('source.updated', {})).toBe(false)
  })

  it('fetches a full week when a week panel is present', () => {
    expect(daysToFetch(snap({ agenda_days: 2 }).settings)).toBe(2)
    expect(
      daysToFetch(snap({ agenda_days: 2, panels: [{ kind: 'week' }] as never }).settings),
    ).toBe(7)
    expect(daysToFetch(snap({ agenda_days: 30 }).settings)).toBe(14)
  })
})
