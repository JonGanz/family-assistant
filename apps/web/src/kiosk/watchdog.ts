export interface WatchdogOptions {
  timeoutMs?: number
  checkMs?: number
  /** Nightly reload time, local clock. */
  nightly?: { hour: number; minute: number }
  reload?: () => void
  now?: () => number
}

export interface Watchdog {
  heartbeat(): void
  stop(): void
}

export function nextNightlyReload(from: number, hour: number, minute: number): number {
  const d = new Date(from)
  d.setHours(hour, minute, 0, 0)
  if (d.getTime() <= from) d.setDate(d.getDate() + 1)
  return d.getTime()
}

/** Reloads the page if no heartbeat arrives within `timeoutMs`, plus once nightly. */
export function startWatchdog(opts: WatchdogOptions = {}): Watchdog {
  const timeoutMs = opts.timeoutMs ?? 120_000
  const checkMs = opts.checkMs ?? 5_000
  const { hour, minute } = opts.nightly ?? { hour: 3, minute: 30 }
  const reload = opts.reload ?? (() => location.reload())
  const now = opts.now ?? (() => Date.now())

  let last = now()
  let nightlyAt = nextNightlyReload(now(), hour, minute)

  const timer = setInterval(() => {
    const t = now()
    if (t - last > timeoutMs || t >= nightlyAt) {
      last = t
      nightlyAt = nextNightlyReload(t, hour, minute)
      reload()
    }
  }, checkMs)

  return {
    heartbeat: () => {
      last = now()
    },
    stop: () => clearInterval(timer),
  }
}
