import { hueVar } from '../../lib/hues'

/** localStorage key holding the runs that already got their confetti. */
export const FIRED_KEY = 'fa.dashboard.confetti.fired'
/** Fired keys older than this many days are dropped. */
export const KEEP_DAYS = 3

/** A daily run is identified by its id plus its date, so a new day's run fires again. */
export const runKey = (run: { id: string; run_date: string }): string => `${run.id}@${run.run_date}`

/** Lighter and darker shades of a member colour (any CSS colour, hue variables included). */
export function shades(color: string): string[] {
  const mix = (other: string, pct: number) => `color-mix(in srgb, ${color} ${pct}%, ${other})`
  return [mix('white', 55), mix('white', 78), color, mix('black', 80), mix('black', 62)]
}

/** The shades of a member's hue (`var(--hue-x)`; unset or unknown is blue). */
export const memberShades = (hue: string | null | undefined): string[] => shades(hueVar(hue))

type Stored = Record<string, string>

function load(): Stored {
  try {
    const raw = localStorage.getItem(FIRED_KEY)
    const v = raw ? (JSON.parse(raw) as unknown) : {}
    return v && typeof v === 'object' && !Array.isArray(v) ? (v as Stored) : {}
  } catch {
    return {}
  }
}

export function hasFired(key: string): boolean {
  return key in load()
}

/** Records a key (with its run date) and prunes entries older than `KEEP_DAYS`. */
export function markFired(key: string, runDate: string, today: string): void {
  try {
    const all = load()
    all[key] = runDate
    const cutoff = new Date(`${today}T00:00:00Z`).getTime() - KEEP_DAYS * 86_400_000
    for (const [k, d] of Object.entries(all)) {
      const t = new Date(`${d}T00:00:00Z`).getTime()
      if (Number.isFinite(t) && t < cutoff) delete all[k]
    }
    localStorage.setItem(FIRED_KEY, JSON.stringify(all))
  } catch {
    /* storage unavailable: the in-memory guard still prevents repeats this session */
  }
}

export interface RunState {
  key: string
  runDate: string
  done: boolean
}

/**
 * Compares the previous done-state of each run with the current one and returns the runs that
 * just became done and have not fired yet. A run first seen already done (or first seen at all)
 * never fires: `prev` is seeded from the first snapshot.
 */
export function newlyCompleted(
  prev: Map<string, boolean>,
  runs: RunState[],
  fired: (key: string) => boolean = hasFired,
): RunState[] {
  const out: RunState[] = []
  for (const r of runs) {
    const was = prev.get(r.key)
    if (was === false && r.done && !fired(r.key)) out.push(r)
    prev.set(r.key, r.done)
  }
  return out
}

/** Effects are skipped for people who ask the system for less motion. */
export const reducedMotion = (): boolean =>
  typeof window !== 'undefined' &&
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches

export interface ConfettiRequest {
  /** The viewport rectangle of the card to burst from. */
  rect: { left: number; top: number; width: number; height: number }
  colors: string[]
  /** Extra bursts at random places on the whole screen, staggered (see `scatterPlan`). */
  scatter?: number
}

export const SCATTER_MIN = 5
export const SCATTER_MAX = 8
/** The scattered bursts are spread over this long. */
export const SCATTER_SPAN_MS = 2000

/** How many scattered bursts a completion fires (5 to 8). */
export const scatterCount = (rnd: () => number = Math.random): number =>
  SCATTER_MIN + Math.floor(rnd() * (SCATTER_MAX - SCATTER_MIN + 1))

/**
 * `n` burst centres at random places within a `w` x `h` screen (kept off the very edge), with
 * delays spread evenly over `SCATTER_SPAN_MS` and sorted ascending.
 */
export function scatterPlan(
  n: number,
  w: number,
  h: number,
  rnd: () => number = Math.random,
): { x: number; y: number; delay: number }[] {
  const out = []
  for (let i = 0; i < n; i++) {
    out.push({
      x: Math.round(w * (0.08 + rnd() * 0.84)),
      y: Math.round(h * (0.08 + rnd() * 0.84)),
      delay: Math.round(((i + rnd() * 0.8) / Math.max(1, n)) * SCATTER_SPAN_MS),
    })
  }
  return out
}

export interface Box {
  left: number
  top: number
  width: number
  height: number
}

/** Cards of completed runs that keep a confetti stream rising from behind them. */
export interface StreamSource {
  key: string
  /** The card's current viewport rectangle, or null when it is gone. */
  rect: () => Box | null
  colors: string[]
}
const sources = new Map<string, StreamSource>()
const streamListeners = new Set<() => void>()
const notify = () => streamListeners.forEach((l) => l())
export const registerStream = (key: string, rect: () => Box | null, colors: string[]): void => {
  sources.set(key, { key, rect, colors })
  notify()
}
export const unregisterStream = (key: string): void => {
  if (sources.delete(key)) notify()
}
export const streamSources = (): StreamSource[] => [...sources.values()]
export const onStreamsChange = (l: () => void): (() => void) => {
  streamListeners.add(l)
  return () => streamListeners.delete(l)
}

/** At most this many stream pieces live at once, over all cards (low-power wall tablets). */
export const STREAM_MAX_PIECES = 36
/** One new piece per card this often. */
export const STREAM_INTERVAL_MS = 420
/** Seconds a stream piece takes to rise and fade. */
export const STREAM_LIFE_S = 6

/**
 * Where a stream piece starts and how far it rises: anywhere across the card's width, in its
 * lower half (hidden behind it), rising past the top edge.
 */
export function streamPiece(
  rect: Box,
  rnd: () => number = Math.random,
): { x: number; y: number; rise: number; sway: number } {
  return {
    x: rect.left + rnd() * rect.width,
    y: rect.top + rect.height * (0.4 + rnd() * 0.5),
    rise: rect.height * 0.5 + 90 + rnd() * 110,
    sway: Math.round(rnd() * 40 - 20),
  }
}

type Listener = (r: ConfettiRequest) => void
const listeners = new Set<Listener>()
export const onConfetti = (l: Listener): (() => void) => {
  listeners.add(l)
  return () => listeners.delete(l)
}
export const fireConfetti = (r: ConfettiRequest): void => listeners.forEach((l) => l(r))
