import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSessionStore } from '../../stores/session'
import MemberAccess from './MemberAccess.vue'
import MembersManager from './MembersManager.vue'

vi.mock('../../api/client', () => ({
  api: {
    me: vi.fn(),
    members: vi.fn(),
    createMember: vi.fn(),
    createAccount: vi.fn(),
    resetPassword: vi.fn(),
    setPin: vi.fn(),
  },
}))
import { api } from '../../api/client'
const m = vi.mocked(api)

const adult = (over: Record<string, unknown> = {}) => ({
  id: 'b',
  display_name: 'Bo',
  role: 'adult',
  has_account: false,
  has_pin: false,
  initial_password: false,
  version: 1,
  ...over,
})
const RouterLink = { template: '<a><slot /></a>' }
const mountAccess = (member: Record<string, unknown>) =>
  mount(MemberAccess, { props: { member: member as never }, global: { stubs: { RouterLink } } })

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  const session = useSessionStore()
  session.me = {
    actor: { kind: 'user', member: { id: 'a', role: 'adult' } },
    household: { name: 'H' },
    must_change_password: false,
  } as never
})

describe('MemberAccess for adults', () => {
  it('offers login creation without a PIN for an adult with no account', () => {
    const w = mountAccess(adult())
    expect(w.find('input[name=login_username]').exists()).toBe(true)
    expect(w.find('input[name=login_password]').exists()).toBe(true)
    expect(w.find('input[name=pin]').exists()).toBe(false)
    expect(w.text()).toContain('set their own PIN')
  })

  it('offers a reset for an adult who still has the initial password', () => {
    const w = mountAccess(adult({ has_account: true, initial_password: true }))
    expect(w.find('input[name=login_password]').exists()).toBe(true)
    expect(w.text()).toContain('Reset password')
    expect(w.find('input[name=pin]').exists()).toBe(false)
  })

  it('shows a claimed adult read-only with an explanation', () => {
    const w = mountAccess(adult({ has_account: true }))
    expect(w.find('[data-testid=claimed-note]').exists()).toBe(true)
    expect(w.find('input').exists()).toBe(false)
    expect(w.find('button').exists()).toBe(false)
  })

  it('creates the login with the minimum adult password length', async () => {
    m.createAccount.mockResolvedValue({} as never)
    m.members.mockResolvedValue([] as never)
    const w = mountAccess(adult())
    await w.find('input[name=login_username]').setValue('bo')
    await w.find('input[name=login_password]').setValue('short')
    await w.find('button').trigger('click')
    expect(m.createAccount).not.toHaveBeenCalled()
    await w.find('input[name=login_password]').setValue('a long enough pw')
    await w.find('button').trigger('click')
    await flushPromises()
    expect(m.createAccount).toHaveBeenCalledWith('b', 'bo', 'a long enough pw')
    expect(w.text()).toContain('must change this password at first sign-in')
  })
})

describe('MembersManager', () => {
  it('hides Edit for a claimed adult and shows it for an unclaimed one', async () => {
    m.members.mockResolvedValue([
      adult({ id: 'a', display_name: 'Me', has_account: true }),
      adult({ id: 'c', display_name: 'Claimed', has_account: true }),
      adult({ id: 'u', display_name: 'Fresh', has_account: true, initial_password: true }),
    ] as never)
    const w = mount(MembersManager, { global: { stubs: { RouterLink } } })
    await flushPromises()
    const edit = (n: string) => w.find(`[data-testid="member-${n}"] button`).exists()
    expect(edit('Me')).toBe(true)
    expect(edit('Fresh')).toBe(true)
    expect(edit('Claimed')).toBe(false)
  })

  it('creates an adult with a login, no PIN, and shows the first sign-in note', async () => {
    m.members.mockResolvedValue([] as never)
    m.createMember.mockResolvedValue({ id: 'n', version: 1 } as never)
    m.createAccount.mockResolvedValue({} as never)
    const w = mount(MembersManager, { global: { stubs: { RouterLink } } })
    await flushPromises()
    await w.find('input[name=new_name]').setValue('Bo')
    await w.find('select[name=new_role]').setValue('adult')
    expect(w.find('input[name=new_pin]').exists()).toBe(false)
    await w.find('input[name=new_username]').setValue('bo')
    await w.find('input[name=new_password]').setValue('a long enough pw')
    await w.find('[data-testid=add-member]').trigger('submit')
    await flushPromises()
    expect(m.createAccount).toHaveBeenCalledWith('n', 'bo', 'a long enough pw')
    expect(m.setPin).not.toHaveBeenCalled()
    expect(w.find('[data-testid=created-note]').text()).toContain('first sign-in')
  })
})
