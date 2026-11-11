import type { EventLike, MemberLike } from './types'

/**
 * The colours an event shows: one per member linked to it (in the household's member order), so an
 * event for two members shows both. An event for nobody shows the fallback. Calendars have no colour.
 */
export function eventColors(
  ev: { members: string[] },
  members: MemberLike[],
  fallback: string,
): string[] {
  const own = [...new Set(members.filter((m) => ev.members.includes(m.id)).map((m) => m.color))]
  return own.length ? own : [fallback]
}

/**
 * The single-colour trim for non-event accents (alerts, task chips, plan entries): the same
 * technique, width (`--ev-stripe-w`, set it with the element's transparent left border) and radius
 * as an event's stripe. `color` may be a CSS variable reference.
 */
export function trimStyle(color: string): Record<string, string> {
  return stripeStyle({ color })
}

/**
 * The left stripe of an event: equal bands, one per colour (one colour is a single band).
 * Every event uses this same rendering so the trim looks identical for one or many members.
 *
 * The bands are painted as a background layer under a transparent left border, not with
 * `border-image` (which turns off `border-radius`), so the outer edge follows the
 * rounded corner. The element's own left border sets the stripe's width through
 * `--ev-stripe-w` (default 4px); a background pattern of its own (busy hatching) goes in `--ev-bg`.
 */
export function stripeStyle(ev: Pick<EventLike, 'color' | 'colors'>): Record<string, string> {
  const colors = ev.colors?.length ? ev.colors : [ev.color]
  const band = 100 / colors.length
  const stops = colors.map((c, i) => `${c} ${i * band}% ${(i + 1) * band}%`).join(', ')
  return {
    borderLeftColor: 'transparent',
    backgroundImage: `linear-gradient(to bottom, ${stops}), var(--ev-bg, none)`,
    backgroundSize: 'var(--ev-stripe-w, 4px) 100%, auto',
    backgroundRepeat: 'no-repeat, repeat',
    backgroundOrigin: 'border-box, padding-box',
    backgroundPosition: 'left top, left top',
  }
}
