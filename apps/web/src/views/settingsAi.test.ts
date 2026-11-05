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
  enabled: true,
  has_api_key: true,
  updated_at: '',
}
const point = (dp: string, on = false) => ({
  decision_point: dp,
  llm_enabled: on,
  llm_threshold: 0.7,
  version_id: 'v',
})

function serve(ep: unknown) {
  m.get.mockImplementation(async (p: string) => {
    if (p.endsWith('/llm-endpoint')) {
      if (!ep) throw new ApiError(404, { code: 'not_configured' })
      return ep
    }
    if (p.endsWith('/models')) return { models: [] }
    if (p.endsWith('/settings')) return { mail_extract_mode: 'fallback', max_input_chars: 6000 }
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

  it('shows Undo for the enabled switch and Undo restores', async () => {
    serve(endpoint)
    m.put.mockResolvedValue({ ...endpoint, enabled: false } as never)
    const w = await mountPage()
    await w.find('[data-testid="endpoint-card"] [role="switch"]').trigger('click')
    await flushPromises()
    expect(m.put.mock.calls[0]![1]).toMatchObject({ enabled: false })
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
    })
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
