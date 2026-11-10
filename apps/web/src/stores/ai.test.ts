import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { useAiStore } from './ai'

vi.mock('../api/http', async (orig) => ({
  ...(await orig<typeof import('../api/http')>()),
  http: { get: vi.fn(), put: vi.fn(), post: vi.fn() },
}))
import { http } from '../api/http'
const m = vi.mocked(http)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
})

describe('ai store', () => {
  it('treats 404 not_configured as no endpoint', async () => {
    m.get.mockImplementation(async (p: string) => {
      if (p.endsWith('/llm-endpoint')) throw new ApiError(404, { code: 'not_configured' })
      if (p.endsWith('/settings')) return { mail_extract_mode: 'fallback', max_input_chars: 6000 }
      if (p.endsWith('/pipeline')) return []
      return { calls: [], stages: [], since: '' }
    })
    const s = useAiStore()
    await s.load()
    expect(s.endpoint).toBeNull()
    expect(s.error).toBeNull()
    expect(s.loaded).toBe(true)
  })

  it('passes api_key through untouched and replaces a stage row', async () => {
    const s = useAiStore()
    m.put.mockResolvedValue({ base_url: 'u' } as never)
    await s.saveEndpoint({
      base_url: 'u',
      model: 'm',
      dialect: 'openai_chat',
      timeout_ms: 1,
      thinking: false,
      enabled: true,
    })
    expect(m.put.mock.calls[0]![1]).not.toHaveProperty('api_key')
    s.pipeline = [
      {
        decision_point: 'email.category',
        llm_enabled: false,
        llm_stage_present: true,
        llm_threshold: 0.7,
        rules_threshold: 0.8,
        version_id: 'v',
      },
    ]
    m.put.mockResolvedValue({
      decision_point: 'email.category',
      llm_enabled: true,
      llm_stage_present: true,
      llm_threshold: 0.8,
      rules_threshold: 0.8,
      version_id: 'w',
    } as never)
    await s.setStage('email.category', { llm_enabled: true, llm_threshold: 0.8 })
    expect(m.put).toHaveBeenLastCalledWith('/api/v1/ai/pipeline/email.category', {
      llm_enabled: true,
      llm_threshold: 0.8,
    })
    expect(s.pipeline[0]!.llm_enabled).toBe(true)
  })

  it('loads the backlog with the rest and on demand', async () => {
    m.get.mockImplementation(async (p: string) => {
      if (p.endsWith('/llm-endpoint')) throw new ApiError(404, { code: 'not_configured' })
      if (p.endsWith('/backlog')) return { awaiting: 12, queued: 3 }
      if (p.endsWith('/settings')) return {}
      if (p.endsWith('/pipeline')) return []
      return { calls: [], stages: [], since: '' }
    })
    const s = useAiStore()
    await s.load()
    await vi.waitFor(() => expect(s.backlog).toEqual({ awaiting: 12, queued: 3 }))
    await s.loadBacklog()
    expect(m.get).toHaveBeenCalledWith('/api/v1/ai/backlog')
  })

  it('loads stats for the given days', async () => {
    m.get.mockResolvedValue({ calls: [], stages: [], since: '' } as never)
    await useAiStore().loadStats(30)
    expect(m.get).toHaveBeenCalledWith('/api/v1/ai/stats?days=30')
  })
})
