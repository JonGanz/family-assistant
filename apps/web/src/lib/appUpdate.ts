/**
 * Picking up a new version after a deploy (D-123, Frontend.md §1.0).
 *
 * The server names its build on every API response (`X-FA-Build`) and on stream connect (`build`
 * event). When it differs from the build this page was made from, an update is pending: the app
 * reloads at the next safe moment (foreground or navigation, never over unsaved form changes), the
 * kiosk as soon as it is idle. In a secure context the service worker is asked to fetch the new
 * version first, so the reload does not come back from its precache.
 */
import { ref, watch } from 'vue'

declare const __APP_COMMIT__: string

/** The build this page was made from; empty when unknown. */
const WEB_BUILD = typeof __APP_COMMIT__ === 'string' ? __APP_COMMIT__.trim() : ''

/** The newest build the server reported, once it differs from ours. */
export const pendingBuild = ref<string | null>(null)

/**
 * Same build? Unknown on either side counts as the same (never reload on missing data), and short and
 * long hashes of one commit match by prefix. A `-dirty` suffix is ignored.
 */
export function sameBuild(
  server: string | null | undefined,
  web: string | null | undefined,
): boolean {
  const norm = (s: string | null | undefined) =>
    (s ?? '')
      .trim()
      .replace(/-dirty$/, '')
      .toLowerCase()
  const a = norm(server)
  const b = norm(web)
  if (!a || !b || a === 'unknown' || b === 'unknown') return true
  return a.startsWith(b) || b.startsWith(a)
}

function devServer(): boolean {
  try {
    // The Vite dev server rebuilds on its own; Vitest also sets DEV but must exercise the logic.
    return !!import.meta.env?.DEV && import.meta.env?.MODE !== 'test'
  } catch {
    return false
  }
}

/** Record what the server says it runs (a response header or the stream's `build` event). */
export function noteServerBuild(commit: string | null | undefined, web: string = WEB_BUILD): void {
  if (!commit || devServer()) return
  if (sameBuild(commit, web)) return
  if (pendingBuild.value !== commit) pendingBuild.value = commit.trim()
}

/** The stream's `build` event data is `{"commit": "..."}`. */
export function noteBuildEvent(data: string): void {
  try {
    const v = JSON.parse(data) as { commit?: unknown }
    if (typeof v.commit === 'string') noteServerBuild(v.commit)
  } catch {
    // Not JSON: ignore.
  }
}

async function registration(): Promise<ServiceWorkerRegistration | undefined> {
  try {
    if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return undefined
    return await navigator.serviceWorker.getRegistration()
  } catch {
    return undefined
  }
}

/** Ask the service worker (secure context only) to look for a new version. Never throws. */
export async function checkServiceWorker(): Promise<void> {
  const reg = await registration()
  try {
    await reg?.update()
  } catch {
    // Offline or the worker script is unreachable: try again later.
  }
}

let reloading = false

/**
 * One automatic reload per build and tab: if the mismatch survives it (a stale web build behind the
 * server, say), reloading again cannot help, so only the snackbar's manual Reload remains.
 */
const TRIED_KEY = 'fa.update.tried'
function autoTried(build: string): boolean {
  try {
    return sessionStorage.getItem(TRIED_KEY) === build
  } catch {
    return false
  }
}
function markTried(build: string): void {
  try {
    sessionStorage.setItem(TRIED_KEY, build)
  } catch {
    // Storage blocked: the guard is best effort.
  }
}

/**
 * Reload into the new version. With a service worker, wait (at most `waitMs`) for the updated worker to
 * take control, so the reload is served the new shell rather than the old precache.
 */
export async function applyUpdate(
  opts: { waitMs?: number; reload?: () => void } = {},
): Promise<void> {
  if (reloading) return
  reloading = true
  const reload = opts.reload ?? (() => location.reload())
  if (pendingBuild.value) markTried(pendingBuild.value)
  const reg = await registration()
  if (reg) {
    const changed = new Promise<void>((resolve) => {
      navigator.serviceWorker.addEventListener('controllerchange', () => resolve(), { once: true })
    })
    try {
      await reg.update()
    } catch {
      // Unreachable: reload anyway; the server's no-cache shell still applies.
    }
    if (reg.installing || reg.waiting) {
      await Promise.race([changed, new Promise((r) => setTimeout(r, opts.waitMs ?? 5000))])
    }
  }
  reload()
}

/**
 * Settings → About "Reload app": drop the service worker's caches (not the offline shopping list cache)
 * and reload, so a phone can always force-refresh without browser menus.
 */
export async function reloadApp(reload: () => void = () => location.reload()): Promise<void> {
  try {
    if (typeof caches !== 'undefined') {
      const names = await caches.keys()
      await Promise.all(names.filter((n) => n !== 'fa-grocery').map((n) => caches.delete(n)))
    }
  } catch {
    // No Cache Storage (plain HTTP): nothing to drop.
  }
  await checkServiceWorker()
  reload()
}

export interface UpdateWatchOptions {
  /** Whether reloading now would lose nothing (no unsaved form, no open dialog, ...). */
  canReload: () => boolean
  /** Kiosk: reload as soon as `canReload` allows; app: only on foreground or navigation. */
  eager?: boolean
  /** Called once when an update becomes pending (the app shows a snackbar). */
  onPending?: (build: string) => void
  /** Hourly service-worker check while open (ms). */
  checkEveryMs?: number
  /** How to apply the update; `applyUpdate` by default (tests inject a spy). */
  apply?: () => void | Promise<void>
}

/**
 * Wire the checks: a service-worker update check on every return to the foreground and periodically;
 * a reload when an update is pending and it is safe. Returns `tryNow` (call it on navigation) and `stop`.
 */
export function watchForUpdates(opts: UpdateWatchOptions): {
  tryNow: () => void
  stop: () => void
} {
  let notified: string | null = null
  const tryNow = () => {
    const b = pendingBuild.value
    if (b && !autoTried(b) && opts.canReload()) void (opts.apply ?? applyUpdate)()
  }
  const onVisible = () => {
    if (document.visibilityState !== 'visible') return
    void checkServiceWorker()
    tryNow()
  }
  document.addEventListener('visibilitychange', onVisible)
  const timer = setInterval(() => void checkServiceWorker(), opts.checkEveryMs ?? 3_600_000)
  // Kiosk: poll the idle condition while an update waits.
  const eagerTimer = opts.eager ? setInterval(tryNow, 5_000) : null
  const stopWatch = watchPending((build) => {
    if (notified !== build) {
      notified = build
      opts.onPending?.(build)
    }
    if (opts.eager) tryNow()
  })
  return {
    tryNow,
    stop: () => {
      document.removeEventListener('visibilitychange', onVisible)
      clearInterval(timer)
      if (eagerTimer) clearInterval(eagerTimer)
      stopWatch()
    },
  }
}

function watchPending(cb: (build: string) => void): () => void {
  return watch(
    pendingBuild,
    (b) => {
      if (b) cb(b)
    },
    { immediate: true },
  )
}
