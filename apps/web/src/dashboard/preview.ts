import { addDays, dayKey, startOfDay } from '../lib/time'
import { api, type Member } from '../api/client'
import type { AdvisoryView, DashboardData, MemberPublic, WeatherView } from './types'

export type PreviewData = Omit<DashboardData, 'settings'>

/** Household data for the editor preview, fetched with the user session (not a device token). */
export async function loadPreviewData(): Promise<PreviewData> {
  const h = await api.household()
  const today = dayKey(new Date(), h.tz)
  const from = startOfDay(h.tz, today).toISOString()
  const to = startOfDay(h.tz, addDays(today, 14)).toISOString()
  const [members, events, weather, advisories] = await Promise.all([
    api.members(),
    api.events(from, to),
    api.weather().catch((): WeatherView | null => null),
    api.advisories().catch((): AdvisoryView[] => []),
  ])
  return {
    household: { name: h.name, tz: h.tz, units: h.units, location: h.location ?? null },
    members: members.map(publicMember),
    events,
    weather,
    advisories,
  }
}

export function publicMember(m: Member): MemberPublic {
  return {
    id: m.id,
    display_name: m.display_name,
    role: m.role,
    avatar: m.avatar ?? null,
    avatar_url: m.avatar_url ?? null,
    color: m.color ?? null,
  }
}
