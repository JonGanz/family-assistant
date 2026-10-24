/**
 * SSE client over `fetch` streaming. EventSource cannot send an Authorization header,
 * and devices authenticate with a bearer token, so we parse text/event-stream ourselves
 * (no polyfill). Reconnects with exponential backoff and resumes via `Last-Event-ID`.
 */
export interface SseMessage {
  id?: string
  event: string
  data: string
}

export type SseStatus = 'connecting' | 'open' | 'reconnecting' | 'closed'

export interface SseOptions {
  url: string
  headers?: () => Record<string, string>
  onMessage: (m: SseMessage) => void
  onStatus?: (s: SseStatus) => void
  fetchImpl?: typeof fetch
  baseDelayMs?: number
  maxDelayMs?: number
  sleep?: (ms: number) => Promise<void>
}

/** Incremental text/event-stream parser. Feed chunks; completed messages are returned. */
export function createSseParser(onLastId?: (id: string) => void) {
  let buf = ''
  let id: string | undefined
  let event = ''
  let data: string[] = []
  const out: SseMessage[] = []

  const dispatch = () => {
    if (data.length) out.push({ id, event: event || 'message', data: data.join('\n') })
    event = ''
    data = []
  }

  return {
    push(chunk: string): SseMessage[] {
      buf += chunk
      const lines = buf.split(/\r\n|\n|\r/)
      buf = lines.pop() ?? ''
      for (const line of lines) {
        if (line === '') {
          dispatch()
          continue
        }
        if (line.startsWith(':')) continue
        const i = line.indexOf(':')
        const field = i < 0 ? line : line.slice(0, i)
        let value = i < 0 ? '' : line.slice(i + 1)
        if (value.startsWith(' ')) value = value.slice(1)
        if (field === 'event') event = value
        else if (field === 'data') data.push(value)
        else if (field === 'id') {
          id = value
          onLastId?.(value)
        }
      }
      return out.splice(0)
    },
  }
}

export function backoffDelay(attempt: number, base = 1000, max = 30_000): number {
  return Math.min(max, base * 2 ** attempt)
}

export interface SseHandle {
  stop(): void
}

export function connectSse(opts: SseOptions): SseHandle {
  const doFetch = opts.fetchImpl ?? ((...a: Parameters<typeof fetch>) => fetch(...a))
  const sleep = opts.sleep ?? ((ms: number) => new Promise<void>((r) => setTimeout(r, ms)))
  const ctl = new AbortController()
  let lastId: string | undefined
  let attempt = 0

  const run = async () => {
    while (!ctl.signal.aborted) {
      opts.onStatus?.(attempt === 0 ? 'connecting' : 'reconnecting')
      try {
        const headers: Record<string, string> = {
          Accept: 'text/event-stream',
          ...opts.headers?.(),
        }
        if (lastId) headers['Last-Event-ID'] = lastId
        const res = await doFetch(opts.url, {
          headers,
          credentials: 'include',
          signal: ctl.signal,
        })
        if (!res.ok || !res.body) throw new Error(`stream HTTP ${res.status}`)
        opts.onStatus?.('open')
        const reader = res.body.getReader()
        const decoder = new TextDecoder()
        const parser = createSseParser((id) => {
          lastId = id
        })
        for (;;) {
          const { done, value } = await reader.read()
          if (done) break
          for (const m of parser.push(decoder.decode(value, { stream: true }))) {
            attempt = 0
            opts.onMessage(m)
          }
        }
      } catch {
        if (ctl.signal.aborted) break
      }
      if (ctl.signal.aborted) break
      opts.onStatus?.('reconnecting')
      await sleep(backoffDelay(attempt++, opts.baseDelayMs, opts.maxDelayMs))
    }
    opts.onStatus?.('closed')
  }
  void run()
  return { stop: () => ctl.abort() }
}
