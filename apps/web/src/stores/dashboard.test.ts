import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'
import type { DashboardData } from '../dashboard/types'

vi.mock('../api/client', () => ({
  api: { me: vi.fn(), dashboard: vi.fn() },
}))
vi.mock('../kiosk/taskApi', () => ({
  taskApi: {
    lists: vi.fn().mockResolvedValue([]),
    household: vi.fn().mockResolvedValue({ now_list_size: 4 }),
    complete: vi.fn(),
    reopen: vi.fn(),
    create: vi.fn(),
    tickStep: vi.fn(),
  },
}))
import { api } from '../api/client'
import { ApiError } from '../api/http'
import { taskApi } from '../kiosk/taskApi'
import { CACHE_KEY, RETRY_MS, daysToFetch, readCache, useDashboardStore } from './dashboard'

const dash = vi.mocked(api.dashboard)

function snap(over: Partial<DashboardData['settings']> = {}): DashboardData {
  return {
    household: {
      name: 'Example Household',
      tz: 'UTC',
      units: 'metric',
      location: null,
      now_list_size: 4,
      completion_override: true,
    },
    members: [],
    events: [],
    advisories: [],
    tasks: [{ id: 't1', title: 'Water the plants' }],
    routines: [],
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

  it('with no cache, shows why the load failed and retries until it succeeds', async () => {
    vi.useFakeTimers()
    try {
      dash.mockResolvedValue(snap())
      dash.mockRejectedValueOnce(new ApiError(500, { status: 500, title: 'Internal error' }))
      const s = useDashboardStore()
      await s.refresh()
      expect(s.snapshot).toBeNull()
      expect(s.loadError).toBe('HTTP 500: Internal error')
      await vi.advanceTimersByTimeAsync(RETRY_MS)
      expect(s.snapshot).not.toBeNull()
      expect(s.loadError).toBeNull()
    } finally {
      vi.useRealTimers()
    }
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

  it.each([
    'tasks.updated',
    'routines.updated',
    'weather.updated',
    'advisory.updated',
    'member.updated',
    'household.updated',
    'resync',
  ])('%s refetches', async (kind) => {
    const s = await loaded()
    expect(s.handleEvent(kind, undefined)).toBe(true)
    await s.refresh()
    expect(dash).toHaveBeenCalled()
  })

  it('device.updated refetches for this device only', async () => {
    const s = await loaded()
    expect(s.handleEvent('device.updated', { id: 'other' })).toBe(false)
    expect(dash).not.toHaveBeenCalled()
    expect(s.handleEvent('device.updated', { id: 'dev-1' })).toBe(true)
  })

  it('inbox.updated refetches the snapshot', async () => {
    const s = await loaded()
    expect(s.handleEvent('inbox.updated', {})).toBe(true)
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

  it('keeps tasks and the Now list size in the snapshot and the offline cache', async () => {
    await loaded()
    expect(readCache()?.snapshot.tasks).toHaveLength(1)
    expect(readCache()?.snapshot.now_list_size).toBe(4)
    setActivePinia(createPinia())
    dash.mockRejectedValue(new Error('offline'))
    const s = useDashboardStore()
    await s.refresh()
    expect(s.stale).toBe(true)
    expect(s.snapshot?.tasks).toHaveLength(1)
  })

  it('a 403 on completion switches the dashboard to read-only', async () => {
    const s = await loaded()
    vi.mocked(taskApi.complete).mockRejectedValue(new ApiError(403, { status: 403 }))
    expect(await s.complete('t1', 'm1')).toBe(false)
    expect(s.snapshot?.completion_enabled).toBe(false)
    expect(s.snapshot?.epoch).toBe(1)
  })

  it('a routine step tick is optimistic and sent as the run member', async () => {
    const s = await loaded()
    s.snapshot!.routines = [
      {
        id: 'r1',
        member: 'm1',
        steps: [{ step: 's1', done_at: null }],
        done: 0,
      } as never,
    ]
    vi.mocked(taskApi.tickStep).mockResolvedValue({} as never)
    await s.tickStep('r1', 's1', true)
    expect(taskApi.tickStep).toHaveBeenCalledWith('r1', 's1', true, 'm1')
  })
})
