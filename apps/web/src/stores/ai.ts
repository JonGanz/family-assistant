import { defineStore } from 'pinia'
import { ref } from 'vue'
import { ApiError, http } from '../api/http'
import type { components } from '../gen/api'

type S = components['schemas']
export type LlmEndpointView = S['LlmEndpointView']
export type LlmEndpointPut = S['LlmEndpointPut']
export type LlmEndpointTest = S['LlmEndpointTest']
export type LlmEndpointTestResult = S['LlmEndpointTestResult']
export type AiSettings = S['AiSettings']
export type AiSettingsPut = S['AiSettingsPut']
export type AiPipelinePoint = S['AiPipelinePoint']
export type AiStats = S['AiStats']

const BASE = '/api/v1/ai'

export const useAiStore = defineStore('ai', () => {
  /** `null` while no endpoint is configured (404 `not_configured`). */
  const endpoint = ref<LlmEndpointView | null>(null)
  const settings = ref<AiSettings | null>(null)
  const pipeline = ref<AiPipelinePoint[]>([])
  const stats = ref<AiStats | null>(null)
  const models = ref<string[]>([])
  const loaded = ref(false)
  const error = ref<string | null>(null)

  async function load(): Promise<void> {
    error.value = null
    try {
      try {
        endpoint.value = await http.get<LlmEndpointView>(`${BASE}/llm-endpoint`)
      } catch (e) {
        if (e instanceof ApiError && e.status === 404) endpoint.value = null
        else throw e
      }
      const [s, p] = await Promise.all([
        http.get<AiSettings>(`${BASE}/settings`),
        http.get<AiPipelinePoint[]>(`${BASE}/pipeline`),
      ])
      settings.value = s
      pipeline.value = p
      loaded.value = true
      if (endpoint.value) void loadModels().catch(() => undefined)
      void loadStats().catch(() => undefined)
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      loaded.value = true
    }
  }

  /** `api_key`: absent keeps the stored key, `null` clears it, a string sets it. */
  async function saveEndpoint(body: LlmEndpointPut): Promise<LlmEndpointView> {
    endpoint.value = await http.put<LlmEndpointView>(`${BASE}/llm-endpoint`, body)
    return endpoint.value
  }

  function testEndpoint(body?: LlmEndpointTest): Promise<LlmEndpointTestResult> {
    return http.post<LlmEndpointTestResult>(`${BASE}/llm-endpoint/test`, body ?? null)
  }

  async function loadModels(): Promise<string[]> {
    const r = await http.get<S['LlmModels']>(`${BASE}/llm-endpoint/models`)
    models.value = r.models
    return r.models
  }

  async function saveSettings(body: AiSettingsPut): Promise<AiSettings> {
    settings.value = await http.put<AiSettings>(`${BASE}/settings`, body)
    return settings.value
  }

  async function setStage(
    dp: string,
    body: { llm_enabled: boolean; llm_threshold: number },
  ): Promise<AiPipelinePoint> {
    const p = await http.put<AiPipelinePoint>(`${BASE}/pipeline/${encodeURIComponent(dp)}`, body)
    pipeline.value = pipeline.value.map((x) => (x.decision_point === dp ? p : x))
    return p
  }

  async function loadStats(days = 7): Promise<void> {
    stats.value = await http.get<AiStats>(`${BASE}/stats?days=${days}`)
  }

  return {
    endpoint,
    settings,
    pipeline,
    stats,
    models,
    loaded,
    error,
    load,
    saveEndpoint,
    testEndpoint,
    loadModels,
    saveSettings,
    setStage,
    loadStats,
  }
})
