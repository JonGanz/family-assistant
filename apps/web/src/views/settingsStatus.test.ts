import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SettingsStatusView from './SettingsStatusView.vue'

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
  useRoute: () => ({ path: '/settings/status', query: {} }),
  onBeforeRouteLeave: vi.fn(),
}))
vi.mock('../api/client', () => ({
  api: { status: vi.fn(), failedJobs: vi.fn(), retryJobs: vi.fn(), household: vi.fn() },
}))
const copy = vi.hoisted(() => vi.fn())
vi.mock('../lib/copy', () => ({ copyText: copy }))
import { api } from '../api/client'
const m = vi.mocked(api)

const failures = {
  dead: 2,
  retrying: 1,
  app_version: '1.2.3',
  llm: {
    purpose: 'mail',
    model: 'm1',
    error_kind: 'timeout',
    at: '2026-11-10T10:00:00Z',
    failed_24h: 4,
    calls_24h: 4,
  },
  groups: [
    {
      kind: 'mail.extract',
      status: 'dead',
      error: 'llm timeout',
      count: 2,
      last_at: '2026-11-10T10:00:00Z',
    },
    {
      kind: 'mail.sync',
      status: 'queued',
      error: '401',
      count: 1,
      last_at: '2026-11-10T09:00:00Z',
    },
  ],
  jobs: [
    job('a', 'mail.extract', 'dead', 'llm timeout\ntail'),
    job('b', 'mail.extract', 'dead', 'llm timeout'),
    job('c', 'mail.sync', 'queued', '401'),
  ],
}
function job(id: string, kind: string, status: string, last_error: string) {
  return {
    id,
    kind,
    status,
    attempts: 5,
    max_attempts: 5,
    last_error,
    created_at: '2026-11-10T08:00:00Z',
    updated_at: '2026-11-10T10:00:00Z',
    run_at: '2026-11-10T10:05:00Z',
    refs: { message_id: 'm-1' },
  }
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  m.household.mockResolvedValue({ version: 1 } as never)
  m.status.mockResolvedValue({
    sources: [],
    weather: { ok: true, fetched_at: null },
    backup: { ok: true, last_at: null },
    queue: { ready: 0, running: 0, dead: 2 },
  } as never)
  m.failedJobs.mockResolvedValue(failures as never)
})

describe('Settings, Status, job errors', () => {
  it('lists error groups, expands jobs and copies a plain-text report', async () => {
    copy.mockResolvedValue(true)
    const w = mount(SettingsStatusView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    expect(m.failedJobs).not.toHaveBeenCalled()
    await w.get('[data-testid="failures-toggle"]').trigger('click')
    await flushPromises()
    expect(w.get('[data-testid="failures-summary"]').text()).toContain('2 failed, 1 retrying')
    expect(w.text()).toContain('mail.extract ×2')
    expect(w.get('[data-testid="llm-problem"]').text()).toContain('4 of 4 calls failed')
    expect(w.findAll('[data-testid="job"]')).toHaveLength(0)
    await w.findAll('[data-testid="group-toggle"]')[0]!.trigger('click')
    expect(w.findAll('[data-testid="job"]')).toHaveLength(2)

    await w.get('[data-testid="copy-report"]').trigger('click')
    await flushPromises()
    const text = copy.mock.calls[0]![0] as string
    expect(text).toContain('App version: 1.2.3')
    expect(text).toContain('mail.extract [failed] x2: llm timeout')
    expect(text).toContain('attempts 5/5')
    expect(text).toContain('message_id=m-1')
  })

  it('shows the text when the clipboard is unavailable and retries a dead group', async () => {
    copy.mockResolvedValue(false)
    m.retryJobs.mockResolvedValue({ requeued: 2 } as never)
    const w = mount(SettingsStatusView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    await w.get('[data-testid="failures-toggle"]').trigger('click')
    await flushPromises()
    await w.get('[data-testid="copy-report"]').trigger('click')
    await flushPromises()
    expect(w.get('textarea').element.value).toContain('Family Assistant job errors')
    await w.get('[data-testid="retry"]').trigger('click')
    await flushPromises()
    expect(m.retryJobs).toHaveBeenCalledWith('mail.extract')
  })
})
