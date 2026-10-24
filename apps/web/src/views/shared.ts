import type { EventView, Member } from '../api/client'
import type { EventLike, MemberLike } from '../lib/types'

const FALLBACK_COLOR = '#2f6fed'

export function memberLikes(members: Member[]): MemberLike[] {
  return members.map((m) => ({
    id: m.id,
    display_name: m.display_name,
    color: m.color ?? '#888888',
    avatar_url: m.avatar_url ?? null,
  }))
}

/** Adapts the API event to the shared EventCard shape (colour fallback, 'from ↗' link). */
export function toEventLike(ev: EventView, members: MemberLike[] = []): EventLike {
  const first = members.find((m) => ev.members.includes(m.id))
  return {
    id: ev.id,
    title: ev.title,
    location: ev.location ?? null,
    start_at: ev.start_at,
    end_at: ev.end_at,
    all_day: ev.all_day,
    start_date: ev.start_date ?? null,
    end_date: ev.end_date ?? null,
    members: ev.members,
    color: ev.color ?? first?.color ?? FALLBACK_COLOR,
    visibility: ev.visibility,
    source_url: ev.external_url ?? null,
  }
}

/** Plain-words message for an API failure. */
export function describeError(e: unknown, fallback = 'Something went wrong.'): string {
  if (e && typeof e === 'object' && 'status' in e) {
    const status = (e as { status: number }).status
    if (status === 413) return 'That file is too large (the limit is 512 KB).'
    if (status === 415) return 'That file type is not supported. Use PNG, JPEG or WebP.'
    if (status === 403) return 'You are not allowed to do that.'
  }
  return e instanceof Error && e.message ? e.message : fallback
}

/** Same-origin paths only: no scheme, no `//host`, no backslashes. */
export function safeReturnTo(v: unknown): string {
  if (typeof v !== 'string') return '/settings/calendars'
  const ok = v.startsWith('/') && !v.startsWith('//') && !v.includes('\\')
  return ok ? v : '/settings/calendars'
}
