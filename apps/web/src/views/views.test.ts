import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { authGuard } from '../router'
import { useSessionStore } from '../stores/session'

vi.mock('../api/client', () => ({
  api: {
    me: vi.fn(),
    setup: vi.fn(),
  },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
})

describe('router guard', () => {
  const to = (auth: 'user' | 'adult') => ({ meta: { auth }, fullPath: '/settings/status' }) as never
  const signIn = (role: string) =>
    m.me.mockResolvedValue({
      actor: { kind: 'user', member: { id: 'x', role } },
      household: { name: 'H' },
    } as never)

  it('lets an adult into settings', async () => {
    signIn('adult')
    expect(await authGuard(to('adult'))).toBe(true)
  })

  it('sends a child away from settings but allows user routes', async () => {
    signIn('child')
    expect(await authGuard(to('adult'))).toEqual({ name: 'home' })
    expect(await authGuard(to('user'))).toBe(true)
  })

  it('sends the signed-out to login with a redirect', async () => {
    m.me.mockRejectedValue(new ApiError(401, {}))
    expect(await authGuard(to('adult'))).toEqual({
      name: 'login',
      query: { redirect: '/settings/status' },
    })
    expect(useSessionStore().isUser).toBe(false)
  })
})
