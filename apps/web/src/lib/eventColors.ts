import type { EventLike, MemberLike } from './types'

/**
 * The colours an event shows. A merged event (the same event from several calendars) shows one
 * colour per copy, falling back to its members' colours when a calendar has none; any other event
 * shows its single colour.
 */
export function eventColors(
  ev: { color?: string | null; colors?: string[]; merged?: boolean; members: string[] },
  members: MemberLike[],
  fallback: string,
): string[] {
  if (ev.merged) {
    const own = ev.colors ?? []
    const fromMembers = members.filter((m) => ev.members.includes(m.id)).map((m) => m.color)
    const palette = [...new Set(own.length ? own : fromMembers)]
    if (palette.length) return palette
  }
  const first = members.find((m) => ev.members.includes(m.id))
  return [ev.color ?? first?.color ?? fallback]
}

/** The left stripe of an event: one solid colour, or equal bands for several. */
export function stripeStyle(ev: Pick<EventLike, 'color' | 'colors'>): Record<string, string> {
  const colors = ev.colors ?? []
  if (colors.length < 2) return { borderLeftColor: ev.color }
  const band = 100 / colors.length
  const stops = colors.map((c, i) => `${c} ${i * band}% ${(i + 1) * band}%`).join(', ')
  return { borderImage: `linear-gradient(to bottom, ${stops}) 1` }
}
