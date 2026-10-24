import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { computed } from 'vue'
import { dispatchLive, parseLive } from './live'
import { useCalendarStore } from './calendar'

vi.mock('../api/client', () => ({ api: { events: vi.fn() } }))
import { api } from '../api/client'
const events = vi.mocked(api.events)

const ev = (id: string) => ({
  id,
  title: id,
  start_at: '2026-10-26T10:00:00Z',
  end_at: '2026-10-26T11:00:00Z',
  all_day: false,
  members: [],
  visibility: 'full',
})

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  events.mockResolvedValue([ev('a')] as never)
})

describe('calendar store', () => {
  it('loads a range once and keys events by id', async () => {
    const s = useCalendarStore()
    await s.ensureRange('2026-10-26', '2026-11-02', 'UTC')
    await s.ensureRange('2026-10-26', '2026-11-02', 'UTC')
    expect(events).toHaveBeenCalledTimes(1)
    expect(events).toHaveBeenCalledWith(
      '2026-10-26T00:00:00.000Z',
      '2026-11-02T00:00:00.000Z',
      undefined,
    )
    expect(s.eventsIn('2026-10-26', '2026-11-02', 'UTC').map((e) => e.id)).toEqual(['a'])
  })

  it('re-renders a view that read the range before it loaded', async () => {
    const s = useCalendarStore()
    const shown = computed(() => s.eventsIn('2026-10-26', '2026-11-02', 'UTC').length)
    const loading = s.ensureRange('2026-10-26', '2026-11-02', 'UTC')
    expect(shown.value).toBe(0)
    await loading
    expect(shown.value).toBe(1)
  })

  it('refetches only ranges that overlap calendar.updated', async () => {
    const s = useCalendarStore()
    await s.ensureRange('2026-10-26', '2026-11-02', 'UTC')
    await s.ensureRange('2026-12-01', '2026-12-08', 'UTC')
    events.mockClear()
    await s.invalidate('calendar.updated', {
      from: '2026-10-28T00:00:00Z',
      to: '2026-10-29T00:00:00Z',
    })
    expect(events).toHaveBeenCalledTimes(1)
    expect(events.mock.calls[0]![0]).toBe('2026-10-26T00:00:00.000Z')
  })

  it('refetches everything on resync and drops events that vanished', async () => {
    const s = useCalendarStore()
    await s.ensureRange('2026-10-26', '2026-11-02', 'UTC')
    await s.ensureRange('2026-12-01', '2026-12-08', 'UTC')
    events.mockClear()
    events.mockResolvedValue([ev('b')] as never)
    await s.invalidate('resync')
    expect(events).toHaveBeenCalledTimes(2)
    expect(s.byId['a']).toBeUndefined()
    expect(s.byId['b']).toBeDefined()
  })

  it('ignores unrelated kinds', async () => {
    const s = useCalendarStore()
    await s.ensureRange('2026-10-26', '2026-11-02', 'UTC')
    events.mockClear()
    await s.invalidate('weather.updated')
    expect(events).not.toHaveBeenCalled()
  })
})

describe('live wiring', () => {
  it('parses known SSE kinds and ignores heartbeats', () => {
    expect(parseLive({ event: 'heartbeat', data: 'x' })).toBeNull()
    expect(parseLive({ event: 'calendar.updated', data: '{"from":"a","to":"b"}' })).toEqual({
      kind: 'calendar.updated',
      data: { from: 'a', to: 'b' },
    })
    expect(parseLive({ event: 'resync', data: '{bad' })?.kind).toBe('resync')
  })

  it('dispatches into the calendar store', async () => {
    const s = useCalendarStore()
    await s.ensureRange('2026-10-26', '2026-11-02', 'UTC')
    events.mockClear()
    await dispatchLive('resync')
    expect(events).toHaveBeenCalledTimes(1)
  })
})
