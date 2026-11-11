import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { authGuard, routes } from '../router'
import { resetSnackbar, useSnackbar } from '../ui/composables/useSnackbar'
import SettingsAiView from './SettingsAiView.vue'

vi.mock('vue-router', async (orig) => ({
  ...(await orig<typeof import('vue-router')>()),
  useRouter: () => ({ push: vi.fn() }),
  useRoute: () => ({ path: '/settings/ai', query: {} }),
  onBeforeRouteLeave: vi.fn(),
}))
vi.mock('../api/client', () => ({ api: { setup: vi.fn(), me: vi.fn() } }))
vi.mock('../api/http', async (orig) => ({
  ...(await orig<typeof import('../api/http')>()),
  http: { get: vi.fn(), put: vi.fn(), post: vi.fn() },
}))
import { api } from '../api/client'
import { http } from '../api/http'
const m = vi.mocked(http)

const endpoint = {
  base_url: 'http://gpu.example.com:11434/v1',
  model: 'qwen3:8b',
  dialect: 'openai_chat',
  timeout_ms: 60000,
  thinking: false,
  enabled: true,
  has_api_key: true,
  updated_at: '',
}
const point = (dp: string, on = false) => ({
  decision_point: dp,
  llm_enabled: on,
  llm_stage_present: true,
  llm_threshold: 0.7,
  rules_threshold: 0.8,
  version_id: 'v',
})
let settingsBody: Record<string, unknown> = {
  mail_extract_mode: 'fallback',
  max_input_chars: 6000,
  mail_mode: 'rules_first',
  catchup_days: 30,
  extract_fallback_cutoff: 0.8,
  event_dedupe_llm: true,
  event_dedupe_threshold: 0.8,
}
let backlogBody = { awaiting: 0, queued: 0 }

function serve(ep: unknown) {
  m.get.mockImplementation(async (p: string) => {
    if (p.endsWith('/llm-endpoint')) {
      if (!ep) throw new ApiError(404, { code: 'not_configured' })
      return ep
    }
    if (p.endsWith('/models')) return { models: [] }
    if (p.endsWith('/settings')) return settingsBody
    if (p.endsWith('/backlog')) return backlogBody
    if (p.endsWith('/pipeline')) return [point('email.category'), point('email.is_bill')]
    return { calls: [], stages: [], since: '' }
  })
}
const mountPage = async () => {
  const w = mount(SettingsAiView, { global: { stubs: { RouterLink: true } } })
  await flushPromises()
  return w
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  resetSnackbar()
  settingsBody = { ...settingsBody, mail_mode: 'rules_first' }
  backlogBody = { awaiting: 0, queued: 0 }
})

describe('Settings AI', () => {
  it('shows the empty state on 404 and opens the form with Set up', async () => {
    serve(null)
    const w = await mountPage()
    expect(w.find('[data-testid="endpoint-card"]').text()).toContain('No language model connected')
    expect(w.find('input[name="base_url"]').exists()).toBe(false)
    await w.find('[data-testid="endpoint-card"] button').trigger('click')
    expect(w.find('input[name="base_url"]').attributes('placeholder')).toBe(
      'http://<gpu-pc>:11434/v1',
    )
    expect(w.find('input[name="model"]').attributes('placeholder')).toBe('qwen3:8b')
  })

  it('sends api_key absent, then null after Clear, then the string after Replace', async () => {
    serve(endpoint)
    m.put.mockResolvedValue(endpoint as never)
    const w = await mountPage()
    const save = async () => {
      await w.find('input[name="timeout"]').setValue(String(Math.random() * 10 + 1))
      await w
        .find('.ui-savebar button.ui-btn--filled, .ui-savebar button:last-child')
        .trigger('click')
      await flushPromises()
    }
    await save()
    expect(m.put.mock.calls[0]![1]).not.toHaveProperty('api_key')
    await w.find('[data-testid="key-clear"]').trigger('click')
    await save()
    expect(m.put.mock.calls[1]![1]).toHaveProperty('api_key', null)
    await w.find('[data-testid="key-replace"]').trigger('click')
    await w.find('input[name="api_key"]').setValue('sekret')
    await save()
    expect(m.put.mock.calls[2]![1]).toHaveProperty('api_key', 'sekret')
  })

  it('saves the thinking switch with the form and keeps it on the enabled switch', async () => {
    serve(endpoint)
    m.put.mockResolvedValue({ ...endpoint, thinking: true } as never)
    const w = await mountPage()
    await w.find('[data-testid="thinking"]').trigger('click')
    await w
      .find('.ui-savebar button.ui-btn--filled, .ui-savebar button:last-child')
      .trigger('click')
    await flushPromises()
    expect(m.put.mock.calls[0]![1]).toMatchObject({ thinking: true })
    m.put.mockResolvedValue({ ...endpoint, thinking: true, enabled: false } as never)
    await w.find('[data-testid="endpoint-card"] [role="switch"]').trigger('click')
    await flushPromises()
    expect(m.put.mock.calls[1]![1]).toMatchObject({ enabled: false, thinking: true })
  })

  it('shows Undo for the enabled switch and Undo restores', async () => {
    serve(endpoint)
    m.put.mockResolvedValue({ ...endpoint, enabled: false } as never)
    const w = await mountPage()
    await w.find('[data-testid="endpoint-card"] [role="switch"]').trigger('click')
    await flushPromises()
    expect(m.put.mock.calls[0]![1]).toMatchObject({ enabled: false, thinking: false })
    const bar = useSnackbar().state.current
    expect(bar?.action?.label).toBe('Undo')
    m.put.mockResolvedValue(endpoint as never)
    await bar!.action!.run()
    await flushPromises()
    expect(m.put.mock.calls[1]![1]).toMatchObject({ enabled: true })
  })

  it('renders a successful test and an unavailable one', async () => {
    serve(endpoint)
    const w = await mountPage()
    m.post.mockResolvedValue({ ok: true, latency_ms: 42, model_found: true } as never)
    await w.find('[data-testid="test"]').trigger('click')
    await flushPromises()
    expect(w.find('[data-testid="test-result"]').text()).toContain('42 ms')
    expect(w.find('[data-testid="test-result"]').text()).toContain('model found')
    m.post.mockResolvedValue({ ok: false, error: 'unavailable' } as never)
    await w.find('[data-testid="test"]').trigger('click')
    await flushPromises()
    expect(w.find('[data-testid="test-result"]').text()).toContain("Can't reach the server")
  })

  it('calls PUT /ai/pipeline/{dp} from a stage switch', async () => {
    serve(endpoint)
    m.put.mockResolvedValue(point('email.category', true) as never)
    const w = await mountPage()
    await w.find('[data-testid="stages-card"] [role="switch"]').trigger('click')
    await flushPromises()
    expect(m.put).toHaveBeenCalledWith('/api/v1/ai/pipeline/email.category', {
      llm_enabled: true,
      llm_threshold: 0.7,
      rules_threshold: 0.8,
    })
  })

  it('in llm_first hides the extract mode and rules thresholds and shows the catch-up window', async () => {
    settingsBody = { ...settingsBody, mail_mode: 'llm_first' }
    serve(endpoint)
    const w = await mountPage()
    expect(w.find('input[name="catchup_days"]').exists()).toBe(true)
    expect(
      w
        .find('[data-testid="extract-card"] [role="radio"], [data-testid="extract-card"] .ui-seg')
        .exists(),
    ).toBe(false)
    expect(w.find('[data-testid="rules-threshold"]').exists()).toBe(false)
  })

  it('in llm_first shows a point without an llm stage as on (D-107)', async () => {
    settingsBody = { ...settingsBody, mail_mode: 'llm_first' }
    serve(endpoint)
    const orig = m.get.getMockImplementation()!
    m.get.mockImplementation(async (p: string) => {
      if (p.endsWith('/pipeline'))
        return [{ ...point('email.category'), llm_stage_present: false }, point('email.is_bill')]
      return orig(p)
    })
    const w = await mountPage()
    const sw = w.findAll('[data-testid="stages-card"] [role="switch"]')
    expect(sw[0]!.attributes('aria-checked')).toBe('true')
    expect(sw[1]!.attributes('aria-checked')).toBe('false')
  })

  it('offers AI only and saves it', async () => {
    serve(endpoint)
    m.put.mockResolvedValue({ ...settingsBody, mail_mode: 'llm_only' } as never)
    const w = await mountPage()
    const radio = w.find('[data-testid="mail-mode-card"] input[value="llm_only"]')
    expect(radio.exists()).toBe(true)
    await radio.setValue(true)
    await flushPromises()
    expect(m.put).toHaveBeenCalledWith(
      '/api/v1/ai/settings',
      expect.objectContaining({ mail_mode: 'llm_only' }),
    )
  })

  it('in rules_first shows rules thresholds and no catch-up window', async () => {
    serve(endpoint)
    const w = await mountPage()
    expect(w.find('input[name="catchup_days"]').exists()).toBe(false)
    expect(w.findAll('[data-testid="rules-threshold"]')).toHaveLength(2)
    expect(w.find('[data-testid="cutoff"]').exists()).toBe(true)
  })

  it('saves the calendar check switch and threshold', async () => {
    serve(endpoint)
    m.put.mockResolvedValue({ ...settingsBody, event_dedupe_threshold: 0.9 } as never)
    const w = await mountPage()
    expect(w.find('[data-testid="dedupe-threshold"]').exists()).toBe(true)
    const r = w.find('[data-testid="dedupe-threshold"]')
    await r.setValue('0.9')
    await r.trigger('change')
    await flushPromises()
    expect(m.put).toHaveBeenCalledWith(
      '/api/v1/ai/settings',
      expect.objectContaining({ event_dedupe_threshold: 0.9 }),
    )
    await w.find('[data-testid="extract-card"] [role="switch"]').trigger('click')
    await flushPromises()
    expect(m.put).toHaveBeenCalledWith(
      '/api/v1/ai/settings',
      expect.objectContaining({ event_dedupe_llm: false }),
    )
  })

  it('switching the mail mode saves instantly with Undo and keeps the other settings', async () => {
    serve(endpoint)
    m.put.mockResolvedValue({ ...settingsBody, mail_mode: 'llm_first' } as never)
    const w = await mountPage()
    await w.find('[data-testid="mail-mode-card"] input[value="llm_first"]').setValue(true)
    await flushPromises()
    expect(m.put).toHaveBeenCalledWith('/api/v1/ai/settings', {
      mail_extract_mode: 'fallback',
      max_input_chars: 6000,
      mail_mode: 'llm_first',
    })
    expect(useSnackbar().state.current?.action?.label).toBe('Undo')
  })

  it('shows the backlog line only when something waits', async () => {
    serve(endpoint)
    let w = await mountPage()
    expect(w.find('[data-testid="backlog"]').exists()).toBe(false)
    backlogBody = { awaiting: 12, queued: 3 }
    w = await mountPage()
    expect(w.find('[data-testid="backlog"]').text()).toBe(
      '12 messages waiting for the AI · 3 queued',
    )
  })

  it('Process now shows with a backlog and an enabled endpoint, posts and reloads', async () => {
    serve(endpoint)
    let w = await mountPage()
    expect(w.find('[data-testid="process-now"]').exists()).toBe(false)
    serve({ ...endpoint, enabled: false })
    backlogBody = { awaiting: 12, queued: 3 }
    w = await mountPage()
    expect(w.find('[data-testid="process-now"]').exists()).toBe(false)
    serve(endpoint)
    w = await mountPage()
    m.post.mockResolvedValue({ released: 3, catchup: true })
    const gets = m.get.mock.calls.length
    await w.find('[data-testid="process-now"]').trigger('click')
    await flushPromises()
    expect(m.post).toHaveBeenCalledWith('/api/v1/ai/backlog/process', null)
    expect(m.get.mock.calls.length).toBeGreaterThan(gets)
    expect(useSnackbar().state.current?.text).toBe('Processing 15 messages now')
  })

  it('shows the empty activity text', async () => {
    serve(endpoint)
    const w = await mountPage()
    expect(w.find('[data-testid="activity-card"]').text()).toContain('No AI activity yet.')
  })
})

describe('guard', () => {
  it('keeps a child out of /settings/ai', async () => {
    const settings = routes.find((r) => r.path === '/settings')!
    const meta = settings.children!.find((c) => c.path === 'ai')!.meta
    expect(meta?.auth).toBe('adult')
    vi.mocked(api.setup).mockResolvedValue({
      needed: false,
      owner_needed: false,
      completed: true,
    } as never)
    vi.mocked(api.me).mockResolvedValue({
      actor: { kind: 'user', member: { id: 'x', role: 'child' } },
      household: { name: 'H' },
    } as never)
    expect(
      await authGuard({ name: 'settings-ai', meta, fullPath: '/settings/ai' } as never),
    ).toEqual({
      name: 'home',
    })
  })
})
