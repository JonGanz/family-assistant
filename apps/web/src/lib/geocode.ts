export interface GeoResult {
  label: string
  lat: number
  lon: number
  tz: string | null
}

interface OpenMeteoHit {
  name: string
  latitude: number
  longitude: number
  country?: string
  admin1?: string
  timezone?: string
}

/** City search through Open-Meteo's geocoding API, called from the browser (no key needed). */
export async function searchCities(q: string, fetchFn: typeof fetch = fetch): Promise<GeoResult[]> {
  const name = q.trim()
  if (name.length < 2) return []
  const url = `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(name)}&count=5`
  const res = await fetchFn(url)
  if (!res.ok) throw new Error(`Search failed (${res.status})`)
  const body = (await res.json()) as { results?: OpenMeteoHit[] }
  return (body.results ?? []).map((h) => ({
    label: [h.name, h.admin1, h.country].filter(Boolean).join(', '),
    lat: h.latitude,
    lon: h.longitude,
    tz: h.timezone ?? null,
  }))
}
