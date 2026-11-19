/** Per-recipe tick state for the kiosk recipe view, kept in localStorage and expiring after 12 hours. */
export const CHECKS_PREFIX = 'fa.recipe.checks2.'
/** The index-keyed v1 entries (before D-124); dropped on prune. */
const OLD_PREFIX = 'fa.recipe.checks.'
export const CHECKS_TTL_MS = 12 * 60 * 60_000

export interface Checks {
  lines: string[]
  steps: string[]
  /** Epoch ms of the last change. */
  at: number
}

export const emptyChecks = (): Checks => ({ lines: [], steps: [], at: 0 })

function parse(raw: string | null, now: number): Checks | null {
  if (!raw) return null
  try {
    const v = JSON.parse(raw) as Partial<Checks> | null
    if (!v || typeof v.at !== 'number' || !Array.isArray(v.lines) || !Array.isArray(v.steps))
      return null
    if (now - v.at > CHECKS_TTL_MS) return null
    const ids = (a: unknown[]) => a.filter((n): n is string => typeof n === 'string')
    return { lines: ids(v.lines), steps: ids(v.steps), at: v.at }
  } catch {
    return null
  }
}

/** Loads one recipe's ticks (empty when absent, corrupt or expired; expired entries are removed). */
export function loadChecks(id: string, now = Date.now()): Checks {
  try {
    const raw = localStorage.getItem(CHECKS_PREFIX + id)
    const v = parse(raw, now)
    if (!v && raw !== null) localStorage.removeItem(CHECKS_PREFIX + id)
    return v ?? emptyChecks()
  } catch {
    return emptyChecks()
  }
}

export function saveChecks(id: string, c: Checks): void {
  try {
    if (!c.lines.length && !c.steps.length) localStorage.removeItem(CHECKS_PREFIX + id)
    else localStorage.setItem(CHECKS_PREFIX + id, JSON.stringify(c))
  } catch {
    /* storage unavailable: ticks last for this view only */
  }
}

/** Returns a copy with the line or step `id` of `kind` toggled and the timestamp refreshed. */
export function toggleCheck(
  c: Checks,
  kind: 'lines' | 'steps',
  id: string,
  now = Date.now(),
): Checks {
  const cur = c[kind]
  const next = cur.includes(id) ? cur.filter((n) => n !== id) : [...cur, id]
  return { ...c, [kind]: next, at: now }
}

/** Removes every expired or corrupt recipe entry. */
export function pruneChecks(now = Date.now()): void {
  try {
    const keys: string[] = []
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i)
      if (k?.startsWith(CHECKS_PREFIX) || k?.startsWith(OLD_PREFIX)) keys.push(k)
    }
    for (const k of keys)
      if (k.startsWith(OLD_PREFIX) || !parse(localStorage.getItem(k), now))
        localStorage.removeItem(k)
  } catch {
    /* ignore */
  }
}
