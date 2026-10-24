/** SSE kinds from Feeds.md §6 plus the M0 kinds. Wiring to the SSE client is T-11. */
export type LiveEventKind =
  | 'calendar.updated'
  | 'weather.updated'
  | 'advisory.updated'
  | 'household.updated'
  | 'device.updated'
  | 'source.updated'
  | 'member.updated'
  | 'resync'

/** What a store/snapshot currently holds; `from`/`to` are day keys or ISO instants, `to` exclusive. */
export interface HeldRange {
  kind: 'calendar' | 'weather' | 'advisory' | 'household' | 'device' | 'source' | 'member'
  from?: string
  to?: string
  /** For device-scoped data. */
  id?: string
}

export interface LiveEventData {
  from?: string
  to?: string
  id?: string
  state?: string
}

/** Pure rule: should a store holding `held` refetch after this event? */
export function invalidates(
  kind: LiveEventKind,
  data: LiveEventData | undefined,
  held: HeldRange,
): boolean {
  switch (kind) {
    case 'resync':
      return true
    case 'calendar.updated': {
      if (held.kind !== 'calendar') return false
      if (!data?.from || !data.to || !held.from || !held.to) return true
      return data.from < held.to && held.from < data.to
    }
    case 'weather.updated':
      return held.kind === 'weather'
    case 'advisory.updated':
      return held.kind === 'advisory'
    case 'household.updated':
      return held.kind === 'household' || held.kind === 'calendar' || held.kind === 'weather'
    case 'member.updated':
      return held.kind === 'member'
    case 'device.updated':
      return held.kind === 'device' && (!data?.id || !held.id || data.id === held.id)
    case 'source.updated':
      return held.kind === 'source'
  }
}
