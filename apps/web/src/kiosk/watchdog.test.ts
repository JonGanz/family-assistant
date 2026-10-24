import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { startWatchdog } from './watchdog'

describe('watchdog', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 0, 10, 12, 0, 0))
  })
  afterEach(() => vi.useRealTimers())

  it('reloads after 120 s without heartbeat', () => {
    const reload = vi.fn()
    const w = startWatchdog({ reload })
    vi.advanceTimersByTime(110_000)
    expect(reload).not.toHaveBeenCalled()
    vi.advanceTimersByTime(15_000)
    expect(reload).toHaveBeenCalledTimes(1)
    w.stop()
  })

  it('heartbeats postpone the reload', () => {
    const reload = vi.fn()
    const w = startWatchdog({ reload })
    for (let i = 0; i < 10; i++) {
      vi.advanceTimersByTime(60_000)
      w.heartbeat()
    }
    expect(reload).not.toHaveBeenCalled()
    w.stop()
  })

  it('reloads nightly at 03:30 local time', () => {
    const reload = vi.fn()
    const w = startWatchdog({ reload })
    // 12:00 -> 03:29, heartbeating throughout
    const target = new Date(2026, 0, 11, 3, 29, 0).getTime()
    while (Date.now() < target) {
      vi.advanceTimersByTime(60_000)
      w.heartbeat()
    }
    expect(reload).not.toHaveBeenCalled()
    vi.advanceTimersByTime(90_000)
    expect(reload).toHaveBeenCalledTimes(1)
    w.stop()
  })
})
