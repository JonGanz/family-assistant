import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SettingsSystemView from '../SettingsSystemView.vue'
import { buildPatch, connectionText, draftFrom } from './settingsModel'

vi.mock('../../api/client', () => ({
  api: { systemSettings: vi.fn(), patchSystemSettings: vi.fn(), readyz: vi.fn() },
}))
import { api } from '../../api/client'
const m = vi.mocked(api)

const view = (over: Record<string, unknown> = {}) =>
  ({
    connection: { secure: false, origin: 'http://h.lan:8080' },
    key_backup_confirmed_at: null,
    settings: [
      {
        key: 'public_base_url',
        kind: 'url',
        value: 'http://h.lan:8080',
        is_set: true,
        source: 'db',
        locked: false,
      },
      {
        key: 'ntfy_url',
        kind: 'url',
        value: 'http://ntfy:80',
        is_set: true,
        source: 'env',
        locked: true,
      },
      { key: 'ntfy_token', kind: 'secret', value: null, is_set: true, source: 'db', locked: false },
      {
        key: 'allow_http_feeds',
        kind: 'bool',
        value: false,
        is_set: false,
        source: 'default',
        locked: false,
      },
      {
        key: 'trusted_proxies',
        kind: 'cidrs',
        value: ['127.0.0.0/8'],
        is_set: false,
        source: 'default',
        locked: false,
      },
    ],
    ...over,
  }) as never

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
})

describe('system settings model', () => {
  const s = (view() as { settings: never[] }).settings

  it('only sends changed, editable values', () => {
    const drafts = Object.fromEntries(s.map((x) => [(x as { key: string }).key, draftFrom(x)]))
    expect(buildPatch(s, drafts)).toEqual({})
    drafts.public_base_url!.value = 'http://new.lan'
    drafts.ntfy_url!.value = 'http://other'
    drafts.allow_http_feeds!.value = true
    drafts.trusted_proxies!.value = '10.0.0.0/8\n127.0.0.0/8'
    expect(buildPatch(s, drafts)).toEqual({
      public_base_url: 'http://new.lan',
      allow_http_feeds: true,
      trusted_proxies: ['10.0.0.0/8', '127.0.0.0/8'],
    })
  })

  it('emptying a field resets it, secrets are write-only', () => {
    const drafts = Object.fromEntries(s.map((x) => [(x as { key: string }).key, draftFrom(x)]))
    drafts.public_base_url!.value = ''
    drafts.ntfy_token!.value = 'secret'
    expect(buildPatch(s, drafts)).toEqual({ public_base_url: null, ntfy_token: 'secret' })
    drafts.ntfy_token!.value = ''
    drafts.ntfy_token!.clear = true
    expect(buildPatch(s, drafts).ntfy_token).toBeNull()
  })

  it('states the connection as a fact', () => {
    expect(connectionText({ secure: false, origin: 'http://h.lan:8080' })).toBe(
      'This page is served over HTTP (http://h.lan:8080).',
    )
    expect(connectionText({ secure: true, origin: 'https://family.x.ts.net' })).toContain(
      'Tailscale',
    )
  })
})

describe('System view', () => {
  it('locks fields set by the server and never shows a secret value', async () => {
    m.systemSettings.mockResolvedValue(view())
    const w = mount(SettingsSystemView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    const locked = w.get('[data-testid="setting-ntfy_url"]')
    expect(locked.get('input').attributes('disabled')).toBeDefined()
    expect(locked.get('[data-testid="locked-note"]').text()).toContain('Set by the server')
    expect(w.get('input[name=public_base_url]').attributes('disabled')).toBeUndefined()
    expect(w.get('[data-testid="secret-state-ntfy_token"]').text()).toBe('set')
    expect((w.get('input[name=ntfy_token]').element as HTMLInputElement).value).toBe('')
    expect(w.get('[data-testid="connection-text"]').text()).toContain('HTTP')
  })

  it('warns until the master key backup is confirmed', async () => {
    m.systemSettings.mockResolvedValue(view())
    const w = mount(SettingsSystemView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    expect(w.find('[data-testid="backup-warning"]').exists()).toBe(true)
    m.systemSettings.mockResolvedValue(view({ key_backup_confirmed_at: '2026-10-26T10:00:00Z' }))
    const w2 = mount(SettingsSystemView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    expect(w2.find('[data-testid="backup-warning"]').exists()).toBe(false)
  })

  it('shows applying while it waits for /readyz, then the saved state', async () => {
    m.systemSettings.mockResolvedValue(view())
    m.patchSystemSettings.mockResolvedValue(view())
    m.readyz.mockResolvedValue({} as never)
    const w = mount(SettingsSystemView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    await w.get('input[name=public_base_url]').setValue('http://new.lan')
    await w.get('form[aria-label="Runtime settings"]').trigger('submit')
    await flushPromises()
    expect(m.patchSystemSettings).toHaveBeenCalledWith({ public_base_url: 'http://new.lan' })
    expect(m.readyz).toHaveBeenCalled()
    expect(w.text()).toContain('Saved.')
  })
})
