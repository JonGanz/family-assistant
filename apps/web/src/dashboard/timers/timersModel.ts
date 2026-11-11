// Pure timer model. Running timers keep an absolute `endsAt` (Date.now based), so a throttled or
// suspended page still shows the right remaining time; paused ones keep `remainingMs`.

export type TimerState = 'running' | 'paused' | 'ringing' | 'idle'

export interface Timer {
  id: string
  name: string
  /** The total the ring is drawn against; +1 min extends it. */
  totalMs: number
  state: TimerState
  /** Set while running. */
  endsAt?: number
  /** Set while paused or idle (the time left; idle = the full duration, ready to restart). */
  remainingMs?: number
}

export const MAX_MS = 99 * 3600_000 + 59 * 60_000 + 59_000
export const ADD_MS = 60_000
export const PRESET_MINUTES = [5, 10, 15, 30] as const

export function durationMs(minutes: number, seconds: number): number {
  const m = Math.max(0, Math.floor(Number(minutes) || 0))
  const s = Math.max(0, Math.floor(Number(seconds) || 0))
  return Math.min(MAX_MS, (m * 60 + s) * 1000)
}

let seq = 0
export function newId(now: number): string {
  seq += 1
  return `t${now.toString(36)}${seq}`
}

export function addTimer(list: Timer[], name: string, ms: number, now: number): Timer[] {
  if (ms <= 0) return list
  const t: Timer = {
    id: newId(now),
    name: name.trim(),
    totalMs: ms,
    state: 'running',
    endsAt: now + ms,
  }
  return [...list, t]
}

const map = (list: Timer[], id: string, f: (t: Timer) => Timer) =>
  list.map((t) => (t.id === id ? f(t) : t))

/** Milliseconds left (0 once ringing). */
export function remaining(t: Timer, now: number): number {
  if (t.state === 'running') return Math.max(0, (t.endsAt ?? now) - now)
  if (t.state === 'ringing') return 0
  return Math.max(0, t.remainingMs ?? t.totalMs)
}

/** Running timers whose time is up start ringing. Returns the same array when nothing changed. */
export function tick(list: Timer[], now: number): Timer[] {
  let changed = false
  const out = list.map((t) => {
    if (t.state === 'running' && (t.endsAt ?? 0) <= now) {
      changed = true
      return { ...t, state: 'ringing' as const, endsAt: undefined, remainingMs: 0 }
    }
    return t
  })
  return changed ? out : list
}

export function pause(list: Timer[], id: string, now: number): Timer[] {
  return map(list, id, (t) =>
    t.state === 'running'
      ? { ...t, state: 'paused', remainingMs: remaining(t, now), endsAt: undefined }
      : t,
  )
}

export function resume(list: Timer[], id: string, now: number): Timer[] {
  return map(list, id, (t) => {
    if (t.state === 'paused' || t.state === 'idle') {
      const left = t.remainingMs ?? t.totalMs
      return { ...t, state: 'running', endsAt: now + left, remainingMs: undefined }
    }
    return t
  })
}

/** Add a minute to a running, paused or ringing timer (a ringing one starts again). */
export function addTime(list: Timer[], id: string, now: number, ms = ADD_MS): Timer[] {
  return map(list, id, (t) => {
    if (t.state === 'running')
      return { ...t, endsAt: (t.endsAt ?? now) + ms, totalMs: t.totalMs + ms }
    if (t.state === 'paused' || t.state === 'idle')
      return { ...t, remainingMs: (t.remainingMs ?? t.totalMs) + ms, totalMs: t.totalMs + ms }
    if (t.state === 'ringing')
      return { ...t, state: 'running', endsAt: now + ms, remainingMs: undefined, totalMs: ms }
    return t
  })
}

/** Cancel: stop the run and reset to the full duration (idle); Start runs it again. */
export function cancel(list: Timer[], id: string): Timer[] {
  return map(list, id, (t) => ({ ...t, state: 'idle', endsAt: undefined, remainingMs: t.totalMs }))
}

/** Dismiss a ringing timer: it goes back to idle at its full duration. */
export function dismiss(list: Timer[], id: string): Timer[] {
  return map(list, id, (t) => (t.state === 'ringing' ? cancelOne(t) : t))
}
const cancelOne = (t: Timer): Timer => ({
  ...t,
  state: 'idle',
  endsAt: undefined,
  remainingMs: t.totalMs,
})

export function remove(list: Timer[], id: string): Timer[] {
  return list.filter((t) => t.id !== id)
}

export const anyRinging = (list: Timer[]) => list.some((t) => t.state === 'ringing')

/** 0..1 of the ring still to go. */
export function fraction(t: Timer, now: number): number {
  if (t.totalMs <= 0) return 0
  return Math.min(1, Math.max(0, remaining(t, now) / t.totalMs))
}

/** mm:ss, or h:mm:ss from one hour. Rounds up so 0:00 shows only when it is over. */
export function formatRemaining(ms: number): string {
  const total = Math.ceil(Math.max(0, ms) / 1000)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  const p = (n: number) => String(n).padStart(2, '0')
  return h > 0 ? `${h}:${p(m)}:${p(s)}` : `${p(m)}:${p(s)}`
}

export const STORAGE_KEY = 'fa.dashboard.timers'

export function serialize(list: Timer[]): string {
  return JSON.stringify({ v: 1, timers: list })
}

/** Tolerant: anything malformed yields an empty list; a timer that ended while away rings. */
export function deserialize(raw: string | null, now: number): Timer[] {
  if (!raw) return []
  try {
    const o = JSON.parse(raw) as { v?: number; timers?: unknown }
    if (o?.v !== 1 || !Array.isArray(o.timers)) return []
    const out: Timer[] = []
    for (const x of o.timers as Record<string, unknown>[]) {
      if (!x || typeof x.id !== 'string' || typeof x.totalMs !== 'number') continue
      const state = x.state
      if (state !== 'running' && state !== 'paused' && state !== 'ringing' && state !== 'idle')
        continue
      out.push({
        id: x.id,
        name: typeof x.name === 'string' ? x.name : '',
        totalMs: x.totalMs,
        state,
        endsAt: typeof x.endsAt === 'number' ? x.endsAt : undefined,
        remainingMs: typeof x.remainingMs === 'number' ? x.remainingMs : undefined,
      })
    }
    return tick(out, now)
  } catch {
    return []
  }
}
