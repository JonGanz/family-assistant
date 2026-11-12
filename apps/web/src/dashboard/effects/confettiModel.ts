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
}

type Listener = (r: ConfettiRequest) => void
const listeners = new Set<Listener>()
export const onConfetti = (l: Listener): (() => void) => {
  listeners.add(l)
  return () => listeners.delete(l)
}
export const fireConfetti = (r: ConfettiRequest): void => listeners.forEach((l) => l(r))
