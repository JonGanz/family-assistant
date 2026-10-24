import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  api,
  type CalendarPatch,
  type CalendarView,
  type NewSource,
  type OAuthAccountView,
  type SourcePatch,
  type SourceView,
} from '../api/client'
import { isConflict } from '../api/http'
import type { LiveEventKind } from '../composables/useLiveStream'

export const useSourcesStore = defineStore('sources', () => {
  const sources = ref<SourceView[]>([])
  const calendars = ref<CalendarView[]>([])
  const accounts = ref<OAuthAccountView[]>([])
  const error = ref<string | null>(null)
  const conflict = ref(false)
  const loaded = ref(false)

  async function load(): Promise<void> {
    try {
      const [s, c, a] = await Promise.all([api.sources(), api.calendars(), api.oauthAccounts()])
      sources.value = s
      calendars.value = c
      accounts.value = a
      error.value = null
      conflict.value = false
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  async function guarded<T>(fn: () => Promise<T>): Promise<T | undefined> {
    try {
      const r = await fn()
      conflict.value = false
      return r
    } catch (e) {
      if (isConflict(e)) {
        conflict.value = true
        return undefined
      }
      throw e
    }
  }

  async function create(b: NewSource) {
    const r = await api.createSource(b)
    await load()
    return r
  }

  async function patchSource(s: SourceView, patch: SourcePatch): Promise<boolean> {
    const r = await guarded(() => api.patchSource(s.id, s.version, patch))
    if (r) await load()
    return !!r
  }

  async function patchCalendar(c: CalendarView, patch: CalendarPatch): Promise<boolean> {
    const r = await guarded(() => api.patchCalendar(c.id, c.version, patch))
    if (r) await load()
    return !!r
  }

  async function remove(id: string): Promise<void> {
    await api.deleteSource(id)
    await load()
  }

  async function sync(id: string): Promise<void> {
    await api.syncSource(id)
  }

  async function invalidate(kind: LiveEventKind): Promise<void> {
    if (!loaded.value) return
    if (kind === 'source.updated' || kind === 'calendar.updated' || kind === 'resync') await load()
  }

  return {
    sources,
    calendars,
    accounts,
    error,
    conflict,
    loaded,
    load,
    create,
    patchSource,
    patchCalendar,
    remove,
    sync,
    invalidate,
  }
})
