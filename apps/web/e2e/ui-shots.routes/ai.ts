import type { Page } from '@playwright/test'
import type { components } from '../../src/gen/api'
import type { ShotRoute } from './types'

type S = components['schemas']

/** Fictional LLM-made inbox items and a configured AI page, answered by `page.route`
 *  because the e2e stack has no LLM server. DTO shapes come from the generated types. */
const NOW = new Date().toISOString()
const DAY = 24 * 3600 * 1000
const iso = (n: number) => new Date(Date.now() + n * DAY).toISOString()
const json = (body: unknown) => ({
  status: 200,
  contentType: 'application/json',
  body: JSON.stringify(body),
})

const inbox: S['InboxPage'] = {
  counts: { bill: 1, contact: 0, event: 1, task: 0, total: 2 },
  items: [
    {
      kind: 'event',
      event: {
        all_day: false,
        confidence: 0.82,
        end_at: iso(3),
        extracted_by: 'llm',
        from_label: 'Riverside Swim Club',
        id: '00000000-0000-4000-8000-0000000000e1',
        location: 'Community pool',
        members: [],
        pending_cancel: false,
        start_at: iso(3),
        title: 'Swim meet, warm-up at 8:30',
        version: 1,
      },
    },
    {
      kind: 'bill',
      bill: {
        amount_minor: 4250,
        confidence: 0.76,
        currency: 'USD',
        due_on: iso(10).slice(0, 10),
        extracted_by: 'llm',
        id: '00000000-0000-4000-8000-0000000000b1',
        payee: 'Example Water Utility',
        status: 'proposed',
        task_done: false,
        version: 1,
      },
    },
  ],
}

const endpoint: S['LlmEndpointView'] = {
  base_url: 'http://gpu-pc.example:11434/v1',
  dialect: 'openai_chat',
  enabled: true,
  has_api_key: false,
  model: 'qwen3:8b',
  timeout_ms: 30000,
  updated_at: NOW,
}
const settings: S['AiSettings'] = {
  mail_extract_mode: 'fallback',
  max_input_chars: 8000,
  mail_mode: 'llm_first',
  catchup_days: 30,
  extract_fallback_cutoff: 0.8,
  updated_at: NOW,
}
const pipeline: S['AiPipelinePoint'][] = [
  ['email.category', true],
  ['email.time_sensitive', false],
  ['email.member_link', false],
  ['email.is_bill', false],
].map(([decision_point, llm_enabled]) => ({
  decision_point: decision_point as string,
  llm_enabled: llm_enabled as boolean,
  llm_stage_present: true,
  llm_threshold: 0.7,
  rules_threshold: 0.8,
  version_id: '00000000-0000-4000-8000-000000000001',
}))
const stats: S['AiStats'] = {
  since: iso(-7),
  calls: [
    { purpose: 'mail.llm', total: 42, ok: 40, p50_ms: 1800, p95_ms: 4200 },
    { purpose: 'recipe.extract', total: 3, ok: 3, p50_ms: 5100, p95_ms: 6000 },
  ],
  stages: [
    { decision_point: 'email.category', rules: 118, llm: 24, human: 6 },
    { decision_point: 'email.is_bill', rules: 140, llm: 0, human: 2 },
  ],
}

const backlog: S['LlmBacklog'] = { awaiting: 12, queued: 3 }

async function mockInbox(page: Page) {
  await page.route(/\/api\/v1\/inbox(\?.*)?$/, (r) => r.fulfill(json(inbox)))
}
async function mockAi(page: Page) {
  await page.route(/\/api\/v1\/ai\/llm-endpoint(\?.*)?$/, (r) => r.fulfill(json(endpoint)))
  await page.route(/\/api\/v1\/ai\/llm-endpoint\/models/, (r) =>
    r.fulfill(json({ models: ['qwen3:8b', 'llama3.1:8b'] } satisfies S['LlmModels'])),
  )
  await page.route(/\/api\/v1\/ai\/llm-endpoint\/test/, (r) =>
    r.fulfill(
      json({ ok: true, latency_ms: 640, model_found: true } satisfies S['LlmEndpointTestResult']),
    ),
  )
  await page.route(/\/api\/v1\/ai\/settings/, (r) => r.fulfill(json(settings)))
  await page.route(/\/api\/v1\/ai\/pipeline(\?.*)?$/, (r) => r.fulfill(json(pipeline)))
  await page.route(/\/api\/v1\/ai\/backlog/, (r) => r.fulfill(json(backlog)))
  await page.route(/\/api\/v1\/ai\/stats/, (r) => r.fulfill(json(stats)))
}

export default [
  { route: '/inbox', name: 'ai-inbox', setup: mockInbox },
  {
    route: '/settings/ai',
    name: 'ai-settings',
    setup: mockAi,
    before: async (page) => {
      await page.getByRole('button', { name: 'Test connection' }).click()
      await page.waitForTimeout(600)
    },
  },
] satisfies ShotRoute[]
