import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSessionStore } from '../stores/session'
import { resetSnackbar, useSnackbar } from '../ui/composables/useSnackbar'
import SettingsAboutView from './SettingsAboutView.vue'
import SettingsAccountView from './SettingsAccountView.vue'
import SettingsAppearanceView from './SettingsAppearanceView.vue'
import SettingsIndexView from './SettingsIndexView.vue'

const leaveGuards: (() => unknown)[] = []
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
  useRoute: () => ({ path: '/settings', query: {} }),
  onBeforeRouteLeave: (fn: () => unknown) => leaveGuards.push(fn),
}))
vi.mock('../api/client', () => ({
  api: { changeOwnPassword: vi.fn(), me: vi.fn(), about: vi.fn() },
}))
vi.mock('../api/http', () => ({ request: vi.fn() }))
import { api } from '../api/client'
import { request } from '../api/http'
const m = vi.mocked(api)
const req = vi.mocked(request)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  resetSnackbar()
  localStorage.clear()
  document.documentElement.removeAttribute('data-density')
  leaveGuards.length = 0
})

const session = () => {
  const s = useSessionStore()
  s.me = {
    actor: { kind: 'user', member: { id: 'a', role: 'adult' } },
    household: { name: 'H' },
    must_change_password: false,
  } as never
  return s
}

describe('Appearance: density', () => {
  it('applies at once, stores per device, and Undo puts the old value back', async () => {
    const w = mount(SettingsAppearanceView)
    const radio = (name: string) =>
      w.findAll('[data-testid="density"] [role="radio"]').find((b) => b.text() === name)!
    expect(radio('Comfortable').attributes('aria-checked')).toBe('true')
    await radio('Compact').trigger('click')
    await flushPromises()
    expect(localStorage.getItem('fa.density')).toBe('compact')
    expect(document.documentElement.getAttribute('data-density')).toBe('compact')
    const bar = useSnackbar().state.current
    expect(bar?.text).toBe('Density saved')
    await bar!.action!.run()
    await flushPromises()
    expect(localStorage.getItem('fa.density')).toBe('comfortable')
    expect(radio('Comfortable').attributes('aria-checked')).toBe('true')
  })
})

describe('Account', () => {
  const mountPage = async () => {
    req.mockResolvedValue({ items: [] } as never)
    const w = mount(SettingsAccountView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    return w
  }

  it('shows the Save bar only once the password form is dirty, and has no Log out', async () => {
    session()
    const w = await mountPage()
    expect(w.find('.ui-savebar').exists()).toBe(false)
    expect(w.text()).not.toContain('Log out')
    await w.find('input[name=current_password]').setValue('old')
    expect(w.find('.ui-savebar').exists()).toBe(true)
  })

  it('names a mismatch under the field and saves a valid change, then empties the form', async () => {
    session()
    m.changeOwnPassword.mockResolvedValue(undefined as never)
    m.me.mockResolvedValue(undefined as never)
    const w = await mountPage()
    await w.find('input[name=current_password]').setValue('old-passphrase')
    await w.find('input[name=new_password]').setValue('a long enough passphrase')
    await w.find('input[name=new_password_again]').setValue('something else entirely')
    await w
      .findAll('.ui-savebar button')
      .find((b) => b.text() === 'Save')!
      .trigger('click')
    await flushPromises()
    expect(m.changeOwnPassword).not.toHaveBeenCalled()
    expect(w.text()).toContain('The new passwords do not match.')
    await w.find('input[name=new_password_again]').setValue('a long enough passphrase')
    await w
      .findAll('.ui-savebar button')
      .find((b) => b.text() === 'Save')!
      .trigger('click')
    await flushPromises()
    expect(m.changeOwnPassword).toHaveBeenCalledWith('a', {
      current_password: 'old-passphrase',
      new_password: 'a long enough passphrase',
    })
    expect((w.find('input[name=new_password]').element as HTMLInputElement).value).toBe('')
    expect(w.find('.ui-savebar').exists()).toBe(false)
  })

  it('lists the sessions and revokes one after a confirmation', async () => {
    session()
    req.mockResolvedValue({
      items: [
        {
          id: 's1',
          user_agent: 'Example Browser',
          created_at: '2026-11-01T10:00:00Z',
          last_seen: '2026-11-02T10:00:00Z',
          expires_at: '2026-12-01T10:00:00Z',
        },
      ],
    } as never)
    const w = mount(SettingsAccountView, {
      attachTo: document.body,
      global: { stubs: { RouterLink: true } },
    })
    await flushPromises()
    expect(w.find('[data-testid=sessions]').text()).toContain('Example Browser')
    await w.find('[data-testid=sessions] button').trigger('click')
    const confirm = [...document.body.querySelectorAll('[role=dialog] button')].find(
      (b) => b.textContent === 'Sign out',
    )!
    confirm.dispatchEvent(new Event('click'))
    await flushPromises()
    expect(req).toHaveBeenCalledWith('/api/v1/sessions/s1', { method: 'DELETE', bearer: false })
    w.unmount()
  })
})

describe('Index and About', () => {
  it('the index is a placeholder for the detail pane', () => {
    const w = mount(SettingsIndexView)
    expect(w.text()).toContain('Choose a setting')
  })

  it('About shows the stale-bundle warning card when the commits differ', async () => {
    useSessionStore()
    m.about.mockResolvedValue({
      version: '1.0.0',
      commit: 'aaaaaaa',
      built_at: '2026-10-29T12:00:00Z',
    } as never)
    const w = mount(SettingsAboutView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    expect(w.find('[data-testid=server]').text()).toContain('1.0.0')
    expect(w.find('[data-testid=web]').exists()).toBe(true)
    expect(w.find('[data-testid=stale]').exists()).toBe(true)
  })
})
