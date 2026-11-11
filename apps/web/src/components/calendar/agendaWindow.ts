// Pure windowing logic for the infinite Agenda: fixed-size date chunks, a bounded window of
// them, and merging of the events fetched per chunk. Day keys are YYYY-MM-DD; `to` is exclusive.
import { addDays } from '../../lib/time'

export interface Chunk {
  from: string
  to: string
}
export type Side = 'before' | 'after'

/** Days per fetched chunk, and the most chunks held in memory/DOM at once. */
export const CHUNK_DAYS = 14
export const MAX_CHUNKS = 5

export const chunkAt = (from: string, days = CHUNK_DAYS): Chunk => ({
  from,
  to: addDays(from, days),
})
export const chunkBefore = (c: Chunk, days = CHUNK_DAYS): Chunk => ({
  from: addDays(c.from, -days),
  to: c.from,
})
export const chunkAfter = (c: Chunk, days = CHUNK_DAYS): Chunk => ({
  from: c.to,
  to: addDays(c.to, days),
})

/** The chunk that would be loaded next on `side` of a non-empty window. */
export function nextChunk(win: Chunk[], side: Side, days = CHUNK_DAYS): Chunk {
  return side === 'before' ? chunkBefore(win[0]!, days) : chunkAfter(win[win.length - 1]!, days)
}

/** Adds the next chunk on `side`; chunks beyond `cap` are dropped from the far end. */
export function grow(
  win: Chunk[],
  side: Side,
  cap = MAX_CHUNKS,
  days = CHUNK_DAYS,
): { chunks: Chunk[]; added: Chunk; evicted: Chunk[] } {
  const added = nextChunk(win, side, days)
  const all = side === 'before' ? [added, ...win] : [...win, added]
  const over = Math.max(0, all.length - cap)
  const evicted = side === 'before' ? all.slice(all.length - over) : all.slice(0, over)
  const chunks = side === 'before' ? all.slice(0, all.length - over) : all.slice(over)
  return { chunks, added, evicted }
}

/** The whole span a window covers. */
export const spanOf = (win: Chunk[]): Chunk => ({
  from: win[0]!.from,
  to: win[win.length - 1]!.to,
})

/** Merges per-chunk event lists: an event spanning chunks appears once; sorted by start. */
export function mergeEvents<E extends { id: string; start_at: string }>(lists: E[][]): E[] {
  const byId = new Map<string, E>()
  for (const l of lists) for (const e of l) byId.set(e.id, e)
  return [...byId.values()].sort((a, b) => a.start_at.localeCompare(b.start_at))
}
