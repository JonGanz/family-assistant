import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type AdvisoryView, type WeatherView } from '../api/client'
import { ApiError } from '../api/http'
import type { LiveEventKind } from '../composables/useLiveStream'

export const useWeatherStore = defineStore('weather', () => {
  const latest = ref<WeatherView | null>(null)
  const advisories = ref<AdvisoryView[]>([])
  const error = ref<string | null>(null)
  const loaded = ref(false)

  async function loadWeather(): Promise<void> {
    try {
      latest.value = await api.weather()
    } catch (e) {
      // 404 means no snapshot yet: not an error worth showing.
      if (e instanceof ApiError && e.status === 404) latest.value = null
      else error.value = e instanceof Error ? e.message : String(e)
    }
  }

  async function loadAdvisories(): Promise<void> {
    try {
      advisories.value = await api.advisories()
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  async function load(): Promise<void> {
    error.value = null
    await Promise.all([loadWeather(), loadAdvisories()])
    loaded.value = true
  }

  async function invalidate(kind: LiveEventKind): Promise<void> {
    if (!loaded.value) return
    if (kind === 'resync') await load()
    else if (kind === 'weather.updated' || kind === 'household.updated') await loadWeather()
    if (kind === 'advisory.updated') await loadAdvisories()
  }

  return { latest, advisories, error, loaded, load, invalidate }
})

/** Open-Meteo columnar block: `time[]` plus one array per field (metric). */
export type Columns = { time?: string[] } & Record<string, unknown>

export function columns(v: unknown): Columns {
  return v && typeof v === 'object' ? (v as Columns) : {}
}

export function num(v: unknown): number | undefined {
  return typeof v === 'number' && Number.isFinite(v) ? v : undefined
}

/** One row of a columnar block. */
export function row(c: Columns, i: number): Record<string, number | string | undefined> {
  const out: Record<string, number | string | undefined> = {}
  for (const [k, arr] of Object.entries(c)) {
    if (Array.isArray(arr)) out[k] = arr[i] as number | string | undefined
  }
  return out
}
