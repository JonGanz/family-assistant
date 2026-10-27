import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError, http, onSetupRequired, problemCode } from './http'

describe('http', () => {
  const fetchMock = vi.fn()
  beforeEach(() => {
    localStorage.clear()
    vi.stubGlobal('fetch', fetchMock)
  })
  afterEach(() => {
    fetchMock.mockReset()
    vi.unstubAllGlobals()
  })

  it('sends credentials, CSRF header on mutations and bearer token', async () => {
    localStorage.setItem('fa.deviceToken', 'tok')
    fetchMock.mockResolvedValue(new Response('{}', { status: 200 }))
    await http.post('/api/v1/x', { a: 1 })
    const [, init] = fetchMock.mock.calls[0]
    expect(init.credentials).toBe('include')
    expect(init.headers['X-FA-Request']).toBe('1')
    expect(init.headers['Authorization']).toBe('Bearer tok')
  })

  it('omits CSRF header on GET', async () => {
    fetchMock.mockResolvedValue(new Response('{}', { status: 200 }))
    await http.get('/api/v1/me')
    expect(fetchMock.mock.calls[0][1].headers['X-FA-Request']).toBeUndefined()
  })

  it('parses problem+json into ApiError', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ title: 'Nope', detail: 'bad creds' }), {
        status: 401,
        headers: { 'content-type': 'application/problem+json' },
      }),
    )
    const err = (await http.get('/x').catch((e: unknown) => e)) as ApiError
    expect(err).toBeInstanceOf(ApiError)
    expect(err.status).toBe(401)
    expect(err.message).toBe('bad creds')
  })

  it('calls the setup handler on 409 setup_required', async () => {
    const handler = vi.fn()
    onSetupRequired(handler)
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ code: 'setup_required', detail: 'x' }), {
        status: 409,
        headers: { 'content-type': 'application/problem+json' },
      }),
    )
    const err = await http.get('/api/v1/x').catch((e: unknown) => e)
    expect(handler).toHaveBeenCalledTimes(1)
    expect(problemCode(err)).toBe('setup_required')
    onSetupRequired(null)
  })

  it('does not call it for another 409', async () => {
    const handler = vi.fn()
    onSetupRequired(handler)
    fetchMock.mockResolvedValue(new Response('{}', { status: 409 }))
    await http.get('/api/v1/x').catch(() => undefined)
    expect(handler).not.toHaveBeenCalled()
    onSetupRequired(null)
  })

  it('retries a refused read while the server restarts, but never a write', async () => {
    vi.useFakeTimers()
    fetchMock
      .mockRejectedValueOnce(new TypeError('Failed to fetch'))
      .mockResolvedValueOnce(new Response('{"ok":1}', { status: 200 }))
    const read = http.get('/api/v1/x')
    await vi.advanceTimersByTimeAsync(500)
    await expect(read).resolves.toEqual({ ok: 1 })
    expect(fetchMock).toHaveBeenCalledTimes(2)
    vi.useRealTimers()

    fetchMock.mockReset()
    fetchMock.mockRejectedValueOnce(new TypeError('Failed to fetch'))
    await expect(http.post('/api/v1/x', {})).rejects.toBeInstanceOf(TypeError)
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })
})
