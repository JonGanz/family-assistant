import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  applyUpdate,
  noteBuildEvent,
  noteServerBuild,
  pendingBuild,
  reloadApp,
  sameBuild,
  watchForUpdates,
} from './appUpdate'

afterEach(() => {
  pendingBuild.value = null
  vi.restoreAllMocks()
  vi.useRealTimers()
})

describe('sameBuild (D-123)', () => {
  it('treats unknown builds as the same, so missing data never reloads', () => {
    expect(sameBuild(null, 'abc')).toBe(true)
    expect(sameBuild('abc', '')).toBe(true)
    expect(sameBuild('unknown', 'abc')).toBe(true)
  })
  it('matches short and long hashes of one commit and ignores -dirty', () => {
    expect(sameBuild('abc123def456aa', 'abc123def456')).toBe(true)
    expect(sameBuild('abc123def456-dirty', 'abc123def456')).toBe(true)
    expect(sameBuild('abc123', 'fff999')).toBe(false)
  })
})

describe('noteServerBuild', () => {
  it('marks an update pending only when the server runs a different build', () => {
    noteServerBuild('abc123def456', 'abc123def456')
    expect(pendingBuild.value).toBeNull()
    noteServerBuild('fff999000111', 'abc123def456')
    expect(pendingBuild.value).toBe('fff999000111')
  })
  it('reads the stream build event and ignores bad data', () => {
    noteBuildEvent('not json')
    expect(pendingBuild.value).toBeNull()
  })
})

describe('applyUpdate and reloadApp', () => {
  it('reloads even without a service worker', async () => {
    const reload = vi.fn()
    await applyUpdate({ reload })
    expect(reload).toHaveBeenCalledTimes(1)
  })
  it('Reload app drops every cache except the offline shopping list', async () => {
    const deleted: string[] = []
    vi.stubGlobal('caches', {
      keys: async () => ['workbox-precache-v2', 'fa-avatars', 'fa-grocery'],
      delete: async (n: string) => {
        deleted.push(n)
        return true
      },
    })
    const reload = vi.fn()
    await reloadApp(reload)
    expect(deleted.sort()).toEqual(['fa-avatars', 'workbox-precache-v2'])
    expect(reload).toHaveBeenCalledTimes(1)
    vi.unstubAllGlobals()
  })
})

describe('watchForUpdates', () => {
  it('announces a pending update once and waits while reloading is unsafe', () => {
    const onPending = vi.fn()
    let safe = false
    const w = watchForUpdates({ canReload: () => safe, onPending })
    pendingBuild.value = 'fff999000111'
    return Promise.resolve().then(() => {
      expect(onPending).toHaveBeenCalledTimes(1)
      expect(onPending).toHaveBeenCalledWith('fff999000111')
      safe = false
      w.tryNow() // unsafe: nothing happens (no reload attempted)
      w.stop()
    })
  })
  it('reloads by itself when safe, but only once per build', async () => {
    const apply = vi.fn()
    const w = watchForUpdates({ canReload: () => true, apply })
    pendingBuild.value = 'fff999000111'
    await Promise.resolve()
    w.tryNow()
    expect(apply).toHaveBeenCalledTimes(1)
    sessionStorage.setItem('fa.update.tried', 'fff999000111')
    w.tryNow()
    expect(apply).toHaveBeenCalledTimes(1)
    w.stop()
    sessionStorage.removeItem('fa.update.tried')
  })
})
