import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { useMembersStore } from './members'
import { useSessionStore } from './session'

vi.mock('../api/client', () => ({
  api: {
    me: vi.fn(),
    members: vi.fn(),
    login: vi.fn(),
    logout: vi.fn(),
    patchMember: vi.fn(),
    createMember: vi.fn(),
  },
}))
import { api, type Member } from '../api/client'
const m = vi.mocked(api)

const me = (kind: string, role = 'adult') =>
  ({
    actor: { kind, member: { id: 'm1', role } },
    household: { name: 'Example Household' },
  }) as never

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  localStorage.clear()
})

describe('session store', () => {
  it('loads the actor and derives roles', async () => {
    m.me.mockResolvedValue(me('user'))
    const s = useSessionStore()
    await s.load()
    expect(s.isUser).toBe(true)
    expect(s.isAdult).toBe(true)
    expect(s.household?.name).toBe('Example Household')
  })

  it('treats 401 as logged out and device actors as non-users', async () => {
    m.me.mockRejectedValueOnce(new ApiError(401, {}))
    const s = useSessionStore()
    await s.load()
    expect(s.isUser).toBe(false)
    expect(s.loaded).toBe(true)
    m.me.mockResolvedValue(me('device'))
    await s.load(true)
    expect(s.isUser).toBe(false)
  })

  it('login then reloads /me; logout clears', async () => {
    m.login.mockResolvedValue({} as never)
    m.me.mockResolvedValue(me('user', 'child'))
    const s = useSessionStore()
    await s.login('a@example.com', 'pw')
    expect(s.isUser).toBe(true)
    expect(s.isAdult).toBe(false)
    m.logout.mockResolvedValue(undefined)
    await s.logout()
    expect(s.me).toBeNull()
  })
})

describe('members store', () => {
  it('flags a 409 conflict instead of throwing, and reload clears it', async () => {
    const s = useMembersStore()
    const member: Member = { id: 'x', display_name: 'A', role: 'adult', version: 2 }
    m.patchMember.mockRejectedValue(new ApiError(409, {}))
    expect(await s.update(member, { display_name: 'B' })).toBe(false)
    expect(s.conflictId).toBe('x')
    expect(m.patchMember).toHaveBeenCalledWith('x', 2, { display_name: 'B' })
    m.members.mockResolvedValue([member])
    await s.load()
    expect(s.conflictId).toBeNull()
  })

  it('rethrows other errors', async () => {
    const s = useMembersStore()
    m.patchMember.mockRejectedValue(new ApiError(403, {}))
    await expect(
      s.update({ id: 'x', display_name: 'A', role: 'adult' }, {}),
    ).rejects.toBeInstanceOf(ApiError)
  })
})
