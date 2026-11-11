import { describe, expect, it } from 'vitest'
import { chunkAt, grow, mergeEvents, spanOf, type Chunk } from './agendaWindow'

const contiguous = (w: Chunk[]) => w.every((c, i) => i === 0 || w[i - 1]!.to === c.from)

describe('agendaWindow', () => {
  it('makes contiguous chunks and never exceeds the cap', () => {
    let w = [chunkAt('2026-11-11', 14)]
    for (let i = 0; i < 10; i++) {
      w = grow(w, i % 3 === 0 ? 'before' : 'after', 5, 14).chunks
      expect(w.length).toBeLessThanOrEqual(5)
      expect(contiguous(w)).toBe(true)
    }
  })
  it('evicts from the far end', () => {
    let w = [chunkAt('2026-11-11', 7)]
    for (let i = 0; i < 3; i++) w = grow(w, 'after', 3, 7).chunks
    expect(w.map((c) => c.from)).toEqual(['2026-11-18', '2026-11-25', '2026-12-02'])
    const r = grow(w, 'before', 3, 7)
    expect(r.added.from).toBe('2026-11-11')
    expect(r.evicted).toEqual([{ from: '2026-12-02', to: '2026-12-09' }])
    expect(spanOf(r.chunks)).toEqual({ from: '2026-11-11', to: '2026-12-02' })
  })
  it('dedupes events spanning chunks and sorts by start', () => {
    const a = { id: 'a', start_at: '2026-11-24T10:00:00Z' }
    const b = { id: 'b', start_at: '2026-11-12T10:00:00Z' }
    expect(mergeEvents([[a], [b, a]]).map((e) => e.id)).toEqual(['b', 'a'])
  })
})
