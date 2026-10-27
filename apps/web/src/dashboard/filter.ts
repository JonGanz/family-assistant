import type { DashboardData, DashboardSettings, EventView } from './types'
import type { EventLike, MemberLike } from '../lib/types'
import type { MemberPublic } from './types'

const FALLBACK = '#6b7280'

/**
 * The server returns every member and event; the client applies `settings.members`.
 * Events linked to nobody are household-wide and always shown.
 */
export function visibleEvents(data: DashboardData, settings: DashboardSettings): EventView[] {
  const sel = settings.members
  if (sel === 'all') return data.events
  const ids = new Set<string>(sel)
  return data.events.filter((e) => e.members.length === 0 || e.members.some((m) => ids.has(m)))
}

export function visibleMembers(data: DashboardData, settings: DashboardSettings): MemberPublic[] {
  const sel = settings.members
  if (sel === 'all') return data.members
  const ids = new Set<string>(sel)
  return data.members.filter((m) => ids.has(m.id))
}

/** Pending approvals per member id; members with none (or a switched-off household) are absent. */
export function approvalCounts(data: DashboardData): Record<string, number> {
  const out: Record<string, number> = {}
  for (const p of data.pending_approvals ?? []) if (p.count > 0) out[p.member_id] = p.count
  return out
}

export function toEventLike(e: EventView): EventLike {
  return {
    id: e.id,
    title: e.title,
    location: e.location,
    start_at: e.start_at,
    end_at: e.end_at,
    all_day: e.all_day,
    start_date: e.start_date,
    end_date: e.end_date,
    members: e.members,
    color: e.color ?? FALLBACK,
    visibility: e.visibility,
    source_url: e.external_url,
  }
}

export function toMemberLike(m: MemberPublic): MemberLike {
  return {
    id: m.id,
    display_name: m.display_name,
    color: m.color ?? FALLBACK,
    avatar_url: m.avatar_url,
  }
}
