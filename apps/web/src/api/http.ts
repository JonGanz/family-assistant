import { getDeviceToken } from '../kiosk/deviceToken'

export interface ProblemDetails {
  type?: string
  title?: string
  status?: number
  detail?: string
  instance?: string
  [key: string]: unknown
}

export class ApiError extends Error {
  readonly status: number
  readonly problem: ProblemDetails

  constructor(status: number, problem: ProblemDetails) {
    super(problem.detail ?? problem.title ?? `HTTP ${status}`)
    this.name = 'ApiError'
    this.status = status
    this.problem = problem
  }
}

/** The problem `code` of an error (`setup_required`, `setting_locked`, `invalid_setting`, ...). */
export function problemCode(e: unknown): string | null {
  const c = e instanceof ApiError ? e.problem.code : null
  return typeof c === 'string' ? c : null
}

let setupRequiredHandler: (() => void) | null = null

/** The router registers the redirect to `/setup` for any `409 setup_required` answer. */
export function onSetupRequired(fn: (() => void) | null): void {
  setupRequiredHandler = fn
}

/** True for a 409 (stale `If-Match`) or 428 (missing precondition) response. */
export function isConflict(e: unknown): e is ApiError {
  return e instanceof ApiError && (e.status === 409 || e.status === 428)
}

const SAFE_METHODS = new Set(['GET', 'HEAD', 'OPTIONS'])

export interface RequestOptions {
  method?: string
  body?: unknown
  /** Raw payload sent with its own Content-Type (avatar upload); wins over `body`. */
  raw?: Blob
  headers?: Record<string, string>
  signal?: AbortSignal
  /** Plain-text payload (household TOML import); wins over `body`. */
  text?: string
  /** Read the answer as text instead of JSON (household TOML export). */
  asText?: boolean
  /** Send the stored device token (default true). User-session calls pass false. */
  bearer?: boolean
}

export async function request<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  const method = (opts.method ?? 'GET').toUpperCase()
  const headers: Record<string, string> = { Accept: 'application/json', ...opts.headers }
  if (!SAFE_METHODS.has(method)) headers['X-FA-Request'] = '1'
  const token = opts.bearer === false ? null : getDeviceToken()
  if (token) headers['Authorization'] = `Bearer ${token}`
  let body: string | Blob | undefined
  if (opts.raw) {
    headers['Content-Type'] = opts.raw.type || 'application/octet-stream'
    body = opts.raw
  } else if (opts.text !== undefined) {
    headers['Content-Type'] = 'text/plain'
    body = opts.text
  } else if (opts.body !== undefined) {
    headers['Content-Type'] = 'application/json'
    body = JSON.stringify(opts.body)
  }

  const init: RequestInit = { method, headers, body, credentials: 'include', signal: opts.signal }
  const res = SAFE_METHODS.has(method) ? await fetchRetrying(path, init) : await fetch(path, init)
  if (!res.ok) {
    const problem = await parseProblem(res)
    if (res.status === 409 && problem.code === 'setup_required') setupRequiredHandler?.()
    throw new ApiError(res.status, problem)
  }
  if (res.status === 204) return undefined as T
  if (opts.asText) return (await res.text()) as T
  // Some routes answer 202 with no body (e.g. a queued sync).
  const text = await res.text()
  return (text ? JSON.parse(text) : undefined) as T
}

/**
 * A settings save restarts the server's listeners about a second later, so a
 * read sent then is refused. Reads are safe to repeat; writes are never retried.
 */
export async function fetchRetrying(
  path: string,
  init: RequestInit,
  tries = 6,
  delayMs = 500,
): Promise<Response> {
  for (let i = 1; ; i++) {
    try {
      return await fetch(path, init)
    } catch (e) {
      if (i >= tries || init.signal?.aborted || !(e instanceof TypeError)) throw e
      await new Promise((r) => setTimeout(r, delayMs))
    }
  }
}

async function parseProblem(res: Response): Promise<ProblemDetails> {
  const type = res.headers.get('content-type') ?? ''
  if (type.includes('application/problem+json')) {
    try {
      return { status: res.status, ...((await res.json()) as ProblemDetails) }
    } catch {
      /* fall through */
    }
  }
  return { status: res.status, title: res.statusText || `HTTP ${res.status}` }
}

export const http = {
  get: <T>(path: string, signal?: AbortSignal) => request<T>(path, { signal }),
  post: <T>(path: string, body?: unknown) => request<T>(path, { method: 'POST', body }),
  put: <T>(path: string, body?: unknown) => request<T>(path, { method: 'PUT', body }),
  patch: <T>(path: string, body?: unknown) => request<T>(path, { method: 'PATCH', body }),
  delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
}
