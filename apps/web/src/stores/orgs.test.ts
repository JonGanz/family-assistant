import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../api/mailSettings', () => ({
  mailApi: {
    orgs: vi.fn(),
    enrollments: vi.fn(),
    createOrg: vi.fn(),
    patchOrg: vi.fn(),
    addContact: vi.fn(),
    createContact: vi.fn(),
    createEnrollment: vi.fn(),
    deleteEnrollment: vi.fn(),
  },
}))
import { ApiError } from '../api/http'
import { mailApi } from '../api/mailSettings'
import { useOrgsStore } from './orgs'

const m = vi.mocked(mailApi, true)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  m.orgs.mockResolvedValue([
    { id: 'o1', name: 'Example School', version: 4, contacts: [] },
  ] as never)
})

describe('orgs store', () => {
  it('loads orgs', async () => {
    const s = useOrgsStore()
    await s.load()
    expect(s.orgs[0]?.name).toBe('Example School')
  })

  it('patching sends the org version and reloads', async () => {
    m.patchOrg.mockResolvedValue({} as never)
    const s = useOrgsStore()
    await s.load()
    expect(await s.patchOrg(s.orgs[0]!, { name: 'Renamed' })).toBe(true)
    expect(m.patchOrg).toHaveBeenCalledWith('o1', 4, { name: 'Renamed' })
    expect(m.orgs).toHaveBeenCalledTimes(2)
  })

  it('adding a contact reloads the orgs', async () => {
    m.createContact.mockResolvedValue({} as never)
    const s = useOrgsStore()
    await s.load()
    await s.addContact('o1', { name: 'Teacher', email: 'teacher@example.edu' })
    expect(m.createContact).toHaveBeenCalledWith('o1', {
      name: 'Teacher',
      email: 'teacher@example.edu',
    })
    expect(m.orgs).toHaveBeenCalledTimes(2)
  })

  it('enrollments are kept per member and reloaded after a change', async () => {
    m.enrollments.mockResolvedValue([{ id: 'e1', member: 'k1', version: 1 }] as never)
    m.createEnrollment.mockResolvedValue({} as never)
    const s = useOrgsStore()
    await s.addEnrollment('k1', { org: 'o1', contacts: [] })
    expect(s.enrollments['k1']).toHaveLength(1)
  })

  it('a 409 sets conflict', async () => {
    m.patchOrg.mockRejectedValue(new ApiError(409, { status: 409 }))
    const s = useOrgsStore()
    await s.load()
    expect(await s.patchOrg(s.orgs[0]!, { name: 'x' })).toBe(false)
    expect(s.conflict).toBe(true)
  })
})
