import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  addTime,
  addTimer,
  cancel,
  deserialize,
  dismiss,
  durationMs,
  formatRemaining,
  fraction,
  pause,
  remaining,
  remove,
  resume,
  serialize,
  tick,
} from './timersModel'

describe('timersModel', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(1_000_000)
  })
  afterEach(() => vi.useRealTimers())
  const mk = () => addTimer([], ' Tea ', 60_000, Date.now())

  it('adds a running timer, ignoring zero', () => {
    const l = mk()
    expect(l[0]).toMatchObject({ name: 'Tea', state: 'running', endsAt: 1_060_000 })
    expect(addTimer([], '', 0, 0)).toEqual([])
    expect(durationMs(2, 30)).toBe(150_000)
  })
  it('remaining follows Date.now, not frames', () => {
    const l = mk()
    vi.advanceTimersByTime(20_000)
    expect(remaining(l[0], Date.now())).toBe(40_000)
    expect(fraction(l[0], Date.now())).toBeCloseTo(40 / 60)
  })
  it('tick rings an expired timer and keeps the array otherwise', () => {
    const l = mk()
    expect(tick(l, Date.now())).toBe(l)
    vi.advanceTimersByTime(60_000)
    const r = tick(l, Date.now())
    expect(r[0].state).toBe('ringing')
    expect(remaining(r[0], Date.now())).toBe(0)
  })
  it('pause stores remaining, resume sets a new end', () => {
    let l = mk()
    vi.advanceTimersByTime(10_000)
    l = pause(l, l[0].id, Date.now())
    expect(l[0]).toMatchObject({ state: 'paused', remainingMs: 50_000 })
    vi.advanceTimersByTime(30_000)
    expect(remaining(l[0], Date.now())).toBe(50_000)
    l = resume(l, l[0].id, Date.now())
    expect(remaining(l[0], Date.now())).toBe(50_000)
  })
  it('addTime extends running, paused and ringing timers', () => {
    const a = mk()
    const l = addTime(a, a[0].id, Date.now())
    expect(remaining(l[0], Date.now())).toBe(120_000)
    const p = pause(a, a[0].id, Date.now())
    expect(remaining(addTime(p, p[0].id, Date.now())[0], Date.now())).toBe(120_000)
    vi.advanceTimersByTime(60_000)
    const r = tick(a, Date.now())
    const again = addTime(r, r[0].id, Date.now())
    expect(again[0].state).toBe('running')
    expect(remaining(again[0], Date.now())).toBe(60_000)
  })
  it('cancel and dismiss reset to idle; remove deletes', () => {
    const l = mk()
    expect(cancel(l, l[0].id)[0]).toMatchObject({ state: 'idle', remainingMs: 60_000 })
    vi.advanceTimersByTime(60_000)
    const r = tick(l, Date.now())
    expect(dismiss(r, r[0].id)[0].state).toBe('idle')
    expect(remove(l, l[0].id)).toEqual([])
  })
  it('formats mm:ss and h:mm:ss', () => {
    expect(formatRemaining(65_000)).toBe('01:05')
    expect(formatRemaining(3_600_000)).toBe('1:00:00')
    expect(formatRemaining(0)).toBe('00:00')
    expect(formatRemaining(999)).toBe('00:01')
  })
  it('round-trips and rings a timer that ended while away', () => {
    const l = mk()
    const raw = serialize(l)
    expect(deserialize(raw, Date.now())[0].state).toBe('running')
    expect(deserialize(raw, Date.now() + 120_000)[0].state).toBe('ringing')
    expect(deserialize('garbage', 0)).toEqual([])
    expect(deserialize(null, 0)).toEqual([])
  })
})
