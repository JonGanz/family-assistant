import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { authGuard, routes } from '.'

vi.mock('../api/client', () => ({ api: { setup: vi.fn(), me: vi.fn() } }))
import { api } from '../api/client'
const m = vi.mocked(api)

const status = (over: Record<string, unknown> = {}) => ({
  needed: false,
  owner_needed: false,
  completed: true,
  secure: false,
  origin: 'http://h.lan:8080',
  via_tailscale: false,
  ...over,
})
const signIn = (role: string) =>
  m.me.mockResolvedValue({
    actor: { kind: 'user', member: { id: 'x', role } },
    household: { name: 'H' },
  } as never)
const to = (name: string, auth?: 'user' | 'adult') =>
  ({ name, meta: auth ? { auth } : {}, fullPath: `/${name}` }) as never

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
})

describe('setup redirects', () => {
  it('sends every route to /setup while no household exists', async () => {
    m.setup.mockResolvedValue(
      status({ needed: true, owner_needed: true, completed: false }) as never,
    )
    expect(await authGuard(to('home', 'user'))).toEqual({ name: 'setup' })
    expect(await authGuard(to('login'))).toEqual({ name: 'setup' })
    expect(await authGuard(to('setup'))).toBe(true)
  })

  it('sends every route to /setup while the owner account is missing', async () => {
    m.setup.mockResolvedValue(status({ owner_needed: true, completed: false }) as never)
    expect(await authGuard(to('calendar', 'user'))).toEqual({ name: 'setup' })
  })

  it('keeps an adult in /setup until it is completed', async () => {
    m.setup.mockResolvedValue(status({ completed: false }) as never)
    signIn('adult')
    expect(await authGuard(to('home', 'user'))).toEqual({ name: 'setup' })
    expect(await authGuard(to('setup'))).toBe(true)
  })

  it('does not redirect a child, and a child cannot open /setup', async () => {
    m.setup.mockResolvedValue(status({ completed: false }) as never)
    signIn('child')
    expect(await authGuard(to('home', 'user'))).toBe(true)
    expect(await authGuard(to('setup'))).toEqual({ name: 'home' })
  })

  it('closes /setup once it is completed', async () => {
    m.setup.mockResolvedValue(status() as never)
    signIn('adult')
    expect(await authGuard(to('setup'))).toEqual({ name: 'home' })
    expect(await authGuard(to('home', 'user'))).toBe(true)
  })

  it('does not block the app when the status cannot be read', async () => {
    m.setup.mockRejectedValue(new ApiError(500, {}))
    signIn('adult')
    expect(await authGuard(to('home', 'user'))).toBe(true)
  })
})

describe('routes', () => {
  const paths = (rs: typeof routes, base = ''): string[] =>
    rs.flatMap((r) => [
      `${base}/${r.path}`.replace(/\/+/g, '/'),
      ...paths(r.children ?? [], `${base}/${r.path}`),
    ])

  it('has no /members route (D-070) and puts Members under /settings', () => {
    const all = paths(routes)
    expect(all).not.toContain('/members')
    expect(all).toContain('/settings/members')
  })

  it('guards Members and System for adults only', () => {
    const settings = routes.find((r) => r.path === '/settings')!
    const meta = (p: string) => settings.children!.find((c) => c.path === p)!.meta
    expect(meta('members')?.auth).toBe('adult')
    expect(meta('system')?.auth).toBe('adult')
    expect(meta('account')?.auth).toBe('user')
  })
})
