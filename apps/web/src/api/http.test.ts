import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError, http } from './http'

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
})
