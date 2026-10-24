import { describe, expect, it, vi } from 'vitest'
import { backoffDelay, connectSse, createSseParser, type SseMessage, type SseStatus } from './sse'

function streamOf(chunks: string[], hang = false): Response {
  const enc = new TextEncoder()
  const body = new ReadableStream<Uint8Array>({
    start(c) {
      for (const ch of chunks) c.enqueue(enc.encode(ch))
      if (!hang) c.close()
    },
  })
  return new Response(body, { status: 200 })
}

describe('createSseParser', () => {
  it('parses events split across chunks and ignores comments', () => {
    const p = createSseParser()
    expect(p.push(': hi\nid: 1\nevent: heart')).toEqual([])
    const out = p.push('beat\ndata: x\n\nevent: member.updated\nid: 2\ndata: {"id":"a"}\n\n')
    expect(out).toEqual([
      { id: '1', event: 'heartbeat', data: 'x' },
      { id: '2', event: 'member.updated', data: '{"id":"a"}' },
    ])
  })
})

describe('backoffDelay', () => {
  it('doubles and caps', () => {
    expect(backoffDelay(0)).toBe(1000)
    expect(backoffDelay(3)).toBe(8000)
    expect(backoffDelay(20)).toBe(30_000)
  })
})

describe('connectSse', () => {
  it('reconnects with Last-Event-ID and backoff, resets after success', async () => {
    const msgs: SseMessage[] = []
    const statuses: SseStatus[] = []
    const sleeps: number[] = []
    const calls: Record<string, string>[] = []
    let n = 0
    const done = new Promise<void>((resolve) => {
      const fetchImpl = vi.fn(async (_u: RequestInfo | URL, init?: RequestInit) => {
        calls.push({ ...(init?.headers as Record<string, string>) })
        n++
        if (n === 1) return streamOf(['id: 5\nevent: heartbeat\ndata: a\n\n'])
        if (n === 2) throw new Error('offline')
        if (n === 3) return new Response('no', { status: 503 })
        return streamOf(['id: 6\nevent: heartbeat\ndata: b\n\n'], true)
      }) as unknown as typeof fetch
      const h = connectSse({
        url: '/api/v1/stream',
        headers: () => ({ Authorization: 'Bearer t' }),
        fetchImpl,
        sleep: async (ms) => {
          sleeps.push(ms)
        },
        onStatus: (s) => statuses.push(s),
        onMessage: (m) => {
          msgs.push(m)
          if (m.id === '6') {
            h.stop()
            resolve()
          }
        },
      })
    })
    await done
    expect(msgs.map((m) => m.id)).toEqual(['5', '6'])
    expect(calls[0]['Last-Event-ID']).toBeUndefined()
    expect(calls[1]['Last-Event-ID']).toBe('5')
    expect(calls[3]['Last-Event-ID']).toBe('5')
    expect(calls[3]['Authorization']).toBe('Bearer t')
    // first stream delivered a message (attempt reset) then ended: 1000; failures then double
    expect(sleeps).toEqual([1000, 2000, 4000])
    expect(statuses).toContain('open')
    expect(statuses).toContain('reconnecting')
  })
})
