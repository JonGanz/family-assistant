<script setup lang="ts">
// Settings → AI (Frontend.md §3e). Cards: LLM endpoint (form + Save bar, instant switch), email
// decision stages (instant, Undo), extraction (instant mode + small form), activity (read-only).
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { ApiError } from '../api/http'
import SettingsPage from '../app/SettingsPage.vue'
import {
  useAiStore,
  type AiPipelinePoint,
  type AiSettingsPut,
  type LlmEndpointTestResult,
  type MailMode,
} from '../stores/ai'
import {
  UiButton,
  UiCard,
  UiEmptyState,
  UiErrorState,
  UiIconButton,
  UiList,
  UiListItem,
  UiRadioGroup,
  UiSaveBar,
  UiSegmented,
  UiSelect,
  UiSkeleton,
  UiSwitch,
  UiTextField,
  useFormDraft,
  useInstantSave,
  useSnackbar,
} from '../ui'

const ai = useAiStore()
const snackbar = useSnackbar()

type Dialect = 'openai_chat' | 'ollama_native'
type KeyMode = 'keep' | 'clear' | 'set'
interface EndpointDraft {
  base_url: string
  model: string
  dialect: Dialect
  timeout_s: string
  thinking: boolean
  key_mode: KeyMode
  api_key: string
}

const setupOpen = ref(false)
const showForm = computed(() => ai.endpoint !== null || setupOpen.value)
const fieldErrors = ref<Partial<Record<keyof EndpointDraft, string>>>({})
const FIELD_MAP: Record<string, keyof EndpointDraft> = {
  base_url: 'base_url',
  model: 'model',
  timeout_ms: 'timeout_s',
  api_key: 'api_key',
}

function fromEndpoint(): EndpointDraft {
  const e = ai.endpoint
  return {
    base_url: e?.base_url ?? '',
    model: e?.model ?? '',
    dialect: e?.dialect ?? 'openai_chat',
    timeout_s: String(Math.round((e?.timeout_ms ?? 120_000) / 1000)),
    thinking: e?.thinking ?? false,
    key_mode: 'keep',
    api_key: '',
  }
}

function keyField(d: EndpointDraft): { api_key?: string | null } {
  if (d.key_mode === 'clear') return { api_key: null }
  if (d.key_mode === 'set' && d.api_key !== '') return { api_key: d.api_key }
  return {}
}

const form = useFormDraft(fromEndpoint, async (d) => {
  fieldErrors.value = {}
  const secs = Number(d.timeout_s)
  if (!Number.isFinite(secs) || secs <= 0) {
    fieldErrors.value = { timeout_s: 'Enter a number of seconds.' }
    throw new Error('Check the highlighted field.')
  }
  try {
    await ai.saveEndpoint({
      base_url: d.base_url.trim(),
      model: d.model.trim(),
      dialect: d.dialect,
      timeout_ms: Math.round(secs * 1000),
      thinking: d.thinking,
      enabled: ai.endpoint?.enabled ?? true,
      ...keyField(d),
    })
    setupOpen.value = false
    void ai.loadBacklog().catch(() => undefined)
    void ai.loadModels().catch(() => undefined)
  } catch (e) {
    const field = e instanceof ApiError ? e.problem.field : undefined
    if (typeof field === 'string' && FIELD_MAP[field]) {
      fieldErrors.value = { [FIELD_MAP[field]!]: (e as Error).message }
      throw new Error('Check the highlighted field.', { cause: e })
    }
    throw e
  }
})
const d = form.draft

const dialectOptions = [
  { value: 'openai_chat', label: 'OpenAI-compatible' },
  { value: 'ollama_native', label: 'Ollama native' },
]

// The enabled switch saves the stored values, not the draft.
const enabled = ref(ai.endpoint?.enabled ?? false)
watch(
  () => ai.endpoint?.enabled,
  (v) => (enabled.value = v ?? false),
)
const enabledSave = useInstantSave(
  enabled,
  async (v) => {
    const e = ai.endpoint
    if (!e) return
    await ai.saveEndpoint({
      base_url: e.base_url,
      model: e.model,
      dialect: e.dialect,
      timeout_ms: e.timeout_ms,
      thinking: e.thinking,
      enabled: v,
    })
    void ai.loadBacklog().catch(() => undefined)
  },
  { label: 'Endpoint switch saved' },
)
const backlogText = computed(() => {
  const { awaiting, queued } = ai.backlog
  if (awaiting <= 0 && queued <= 0) return ''
  const w = `${awaiting} ${awaiting === 1 ? 'message' : 'messages'} waiting for the AI`
  return queued > 0 ? `${w} · ${queued} queued` : w
})
const processing = ref(false)
async function processNow() {
  processing.value = true
  try {
    const n = ai.backlog.awaiting + ai.backlog.queued
    await ai.processBacklog()
    snackbar.show({
      text: n > 0 ? `Processing ${n} ${n === 1 ? 'message' : 'messages'} now` : 'Processing now',
    })
    await ai.loadBacklog().catch(() => undefined)
  } catch (e) {
    snackbar.show({ text: e instanceof Error ? e.message : 'Could not start processing' })
  } finally {
    processing.value = false
  }
}
let backlogTimer: ReturnType<typeof setInterval> | undefined

// Model list: a select when the server answers, a text field with Refresh otherwise.
const refreshing = ref(false)
const modelOptions = computed(() => {
  const names =
    ai.models.includes(d.value.model) || !d.value.model ? ai.models : [d.value.model, ...ai.models]
  return names.map((m) => ({ value: m, label: m }))
})
async function refreshModels() {
  refreshing.value = true
  try {
    await ai.loadModels()
  } catch {
    snackbar.show({ text: 'Could not list models from the server.' })
  } finally {
    refreshing.value = false
  }
}

// Test connection: posts the draft values.
const testing = ref(false)
const testResult = ref<LlmEndpointTestResult | null>(null)
const testFailed = ref<string | null>(null)
const ERROR_WORDS: Record<string, string> = {
  unavailable: "Can't reach the server",
  timeout: 'The server did not answer in time',
  http: 'The server answered with an error',
  bad_json: 'The server answered with something unexpected',
  schema: 'The reply did not match the expected format',
}
const testText = computed(() => {
  const r = testResult.value
  if (testFailed.value) return testFailed.value
  if (!r) return ''
  if (!r.ok) return ERROR_WORDS[r.error ?? ''] ?? `Failed: ${r.error ?? 'unknown error'}`
  const ms = r.latency_ms != null ? `${r.latency_ms} ms` : 'ok'
  const found = r.model_found == null ? '' : r.model_found ? ', model found' : ', model not found'
  return `Connected (${ms}${found})`
})
async function testConnection() {
  testing.value = true
  testResult.value = null
  testFailed.value = null
  const secs = Number(d.value.timeout_s)
  try {
    testResult.value = await ai.testEndpoint({
      base_url: d.value.base_url.trim() || null,
      model: d.value.model.trim() || null,
      dialect: d.value.dialect,
      timeout_ms: Number.isFinite(secs) && secs > 0 ? Math.round(secs * 1000) : null,
      ...keyField(d.value),
    })
  } catch (e) {
    testFailed.value = e instanceof Error ? e.message : 'The test could not run.'
  } finally {
    testing.value = false
  }
}

// Email decisions.
const STAGE_NAMES: Record<string, string> = {
  'email.category': 'Category',
  'email.time_sensitive': 'Time-sensitive',
  'email.member_link': 'Which members',
  'email.is_bill': 'Is a bill',
}
const stageName = (dp: string) => STAGE_NAMES[dp] ?? dp
const pct = (v: number) => `${Math.round(v * 100)}%`
const llmFirst = computed(() => (ai.settings?.mail_mode ?? 'llm_first') === 'llm_first')
const thresholds = ref<Record<string, number>>({})
const rulesThresholds = ref<Record<string, number>>({})
watch(
  () => ai.pipeline,
  (rows) => {
    thresholds.value = Object.fromEntries(rows.map((r) => [r.decision_point, r.llm_threshold]))
    rulesThresholds.value = Object.fromEntries(
      rows.map((r) => [r.decision_point, r.rules_threshold]),
    )
  },
  { immediate: true, deep: true },
)
interface StageValues {
  llm_enabled: boolean
  llm_threshold: number
  rules_threshold: number
}
const valuesOf = (p: AiPipelinePoint): StageValues => ({
  llm_enabled: p.llm_enabled,
  llm_threshold: p.llm_threshold,
  rules_threshold: p.rules_threshold,
})
async function applyStage(p: AiPipelinePoint, next: StageValues) {
  const old = valuesOf(p)
  try {
    await ai.setStage(p.decision_point, next)
  } catch (e) {
    thresholds.value[p.decision_point] = old.llm_threshold
    rulesThresholds.value[p.decision_point] = old.rules_threshold
    snackbar.show({ text: e instanceof Error ? e.message : 'Could not save' })
    return
  }
  snackbar.show({
    text: `${stageName(p.decision_point)} saved`,
    action: {
      label: 'Undo',
      run: async () => {
        try {
          await ai.setStage(p.decision_point, old)
        } catch (e) {
          snackbar.show({ text: e instanceof Error ? e.message : 'Could not undo' })
        }
      },
    },
  })
}
// In llm_first a point without an llm stage is still asked, so it reads as on (D-107).
const llmOn = (p: AiPipelinePoint) =>
  llmFirst.value ? !p.llm_stage_present || p.llm_enabled : p.llm_enabled
const toggleStage = (p: AiPipelinePoint, on: boolean) =>
  applyStage(p, { ...valuesOf(p), llm_enabled: on })
function releaseThreshold(p: AiPipelinePoint, ev: Event) {
  const v = Number((ev.target as HTMLInputElement).value)
  if (v !== p.llm_threshold) void applyStage(p, { ...valuesOf(p), llm_threshold: v })
}
function releaseRulesThreshold(p: AiPipelinePoint, ev: Event) {
  const v = Number((ev.target as HTMLInputElement).value)
  if (v !== p.rules_threshold) void applyStage(p, { ...valuesOf(p), rules_threshold: v })
}

// Mail processing and extraction. Every settings save sends the stored values it does not change.
function putSettings(patch: Partial<AiSettingsPut>) {
  const cur = ai.settings
  return ai.saveSettings({
    mail_extract_mode: cur?.mail_extract_mode ?? 'fallback',
    max_input_chars: cur?.max_input_chars ?? 6000,
    ...patch,
  })
}
const mailMode = ref<MailMode>(ai.settings?.mail_mode ?? 'llm_first')
watch(
  () => ai.settings?.mail_mode,
  (v) => v && (mailMode.value = v),
)
const mailModeSave = useInstantSave(
  mailMode,
  async (v) => {
    await putSettings({ mail_mode: v })
  },
  { label: 'Mail processing saved' },
)
const mailModeOptions = [
  { value: 'llm_first', label: 'Use the AI for every message (rules as fallback)' },
  { value: 'rules_first', label: 'Use rules first, AI only when unsure' },
]
const catchupErr = ref('')
const catchupForm = useFormDraft(
  () => ({ catchup_days: String(ai.settings?.catchup_days ?? 30) }),
  async (x) => {
    catchupErr.value = ''
    const n = Number(x.catchup_days)
    if (!Number.isInteger(n) || n < 0 || n > 365) {
      catchupErr.value = 'Enter a whole number from 0 to 365.'
      throw new Error('Check the highlighted field.')
    }
    await putSettings({ catchup_days: n })
  },
)
const catchupHint = computed(() =>
  Number(catchupForm.draft.value.catchup_days) === 0 ? 'Off' : 'Catches up newest first.',
)
const cutoff = ref(ai.settings?.extract_fallback_cutoff ?? 0.8)
watch(
  () => ai.settings?.extract_fallback_cutoff,
  (v) => v != null && (cutoff.value = v),
)
async function releaseCutoff(ev: Event) {
  const v = Number((ev.target as HTMLInputElement).value)
  const old = ai.settings?.extract_fallback_cutoff ?? 0.8
  if (v === old) return
  try {
    await putSettings({ extract_fallback_cutoff: v })
  } catch (e) {
    cutoff.value = old
    snackbar.show({ text: e instanceof Error ? e.message : 'Could not save' })
    return
  }
  snackbar.show({
    text: 'Extraction cutoff saved',
    action: {
      label: 'Undo',
      run: async () => {
        try {
          await putSettings({ extract_fallback_cutoff: old })
        } catch (e) {
          snackbar.show({ text: e instanceof Error ? e.message : 'Could not undo' })
        }
      },
    },
  })
}

const mode = ref(ai.settings?.mail_extract_mode ?? 'fallback')
watch(
  () => ai.settings?.mail_extract_mode,
  (v) => v && (mode.value = v),
)
const modeSave = useInstantSave(
  mode,
  async (v) => {
    await putSettings({ mail_extract_mode: v })
  },
  { label: 'Extraction mode saved' },
)
const modeOptions = [
  { value: 'off', label: 'Off' },
  { value: 'fallback', label: 'When rules find nothing' },
  { value: 'always', label: 'Always' },
]
const maxErr = ref('')
const maxForm = useFormDraft(
  () => ({ max_input_chars: String(ai.settings?.max_input_chars ?? 6000) }),
  async (x) => {
    maxErr.value = ''
    const n = Number(x.max_input_chars)
    if (!Number.isInteger(n) || n <= 0) {
      maxErr.value = 'Enter a whole number.'
      throw new Error('Check the highlighted field.')
    }
    try {
      await putSettings({ max_input_chars: n })
    } catch (e) {
      if (e instanceof ApiError && e.problem.field === 'max_input_chars') {
        maxErr.value = e.message
        throw new Error('Check the highlighted field.', { cause: e })
      }
      throw e
    }
  },
)

// Activity.
const statsBusy = ref(false)
const callRows = computed(() =>
  (ai.stats?.calls ?? []).map((c) => ({
    ...c,
    share: c.total > 0 ? Math.round((c.ok / c.total) * 100) : 0,
  })),
)
const stageRows = computed(() =>
  (ai.stats?.stages ?? []).map((s) => {
    const total = s.rules + s.llm + s.human
    const p = (n: number) => (total > 0 ? (n / total) * 100 : 0)
    return { ...s, total, rp: p(s.rules), lp: p(s.llm), hp: p(s.human) }
  }),
)
const noActivity = computed(
  () => callRows.value.length === 0 && stageRows.value.every((s) => s.total === 0),
)
async function refreshStats() {
  statsBusy.value = true
  try {
    await ai.loadStats()
  } catch {
    snackbar.show({ text: 'Could not load the activity.' })
  } finally {
    statsBusy.value = false
  }
}
const ms = (v?: number | null) => (v == null ? '-' : `${v} ms`)

onMounted(() => {
  void ai.load()
  backlogTimer = setInterval(() => void ai.loadBacklog().catch(() => undefined), 30_000)
})
onUnmounted(() => clearInterval(backlogTimer))
</script>

<template>
  <SettingsPage title="AI">
    <UiErrorState v-if="ai.error" :message="ai.error" @retry="ai.load()" />
    <template v-else-if="!ai.loaded">
      <UiCard v-for="n in 3" :key="n"><UiSkeleton :lines="4" /></UiCard>
    </template>

    <template v-else>
      <UiCard data-testid="endpoint-card">
        <template #header>LLM endpoint</template>
        <UiEmptyState
          v-if="!showForm"
          icon="ai"
          title="No language model connected"
          text="A model running on your own network can read mail the rules cannot sort. Nothing leaves your home."
        >
          <UiButton variant="filled" @click="setupOpen = true">Set up</UiButton>
        </UiEmptyState>
        <template v-else>
          <UiSwitch
            v-if="ai.endpoint"
            :model-value="enabled"
            label="Use this endpoint"
            :loading="enabledSave.saving.value"
            @update:model-value="enabledSave.set($event)"
          />
          <UiTextField
            v-model="d.base_url"
            label="Base URL"
            name="base_url"
            placeholder="http://<gpu-pc>:11434/v1"
            :error="fieldErrors.base_url"
            required
          />
          <UiSegmented v-model="d.dialect" label="Protocol" :options="dialectOptions" />
          <div class="row">
            <UiSelect
              v-if="ai.models.length"
              v-model="d.model"
              label="Model"
              :options="modelOptions"
              :error="fieldErrors.model"
            />
            <UiTextField
              v-else
              v-model="d.model"
              label="Model"
              name="model"
              placeholder="qwen3:8b"
              :error="fieldErrors.model"
              required
            />
            <UiIconButton
              icon="refresh"
              label="Refresh list"
              :disabled="refreshing || !ai.endpoint"
              @click="refreshModels"
            />
          </div>
          <UiTextField
            v-model="d.timeout_s"
            label="Timeout"
            name="timeout"
            type="number"
            suffix="s"
            :error="fieldErrors.timeout_s"
          />
          <UiSwitch v-model="d.thinking" label="Let the model think first" data-testid="thinking" />
          <p class="note">
            Only for models that reason before answering. Sorting and reading mail rarely needs it,
            and each message takes minutes instead of seconds.
          </p>
          <div class="key" data-testid="api-key">
            <template v-if="d.key_mode === 'set'">
              <UiTextField
                v-model="d.api_key"
                label="API key"
                name="api_key"
                type="password"
                :error="fieldErrors.api_key"
              />
              <UiButton variant="text" @click="d.key_mode = 'keep'">Cancel</UiButton>
            </template>
            <template v-else>
              <p class="note">
                API key:
                {{
                  d.key_mode === 'clear'
                    ? 'will be cleared'
                    : ai.endpoint?.has_api_key
                      ? 'set'
                      : 'not set'
                }}
              </p>
              <UiButton variant="text" data-testid="key-replace" @click="d.key_mode = 'set'">{{
                ai.endpoint?.has_api_key ? 'Replace' : 'Set'
              }}</UiButton>
              <UiButton
                v-if="ai.endpoint?.has_api_key && d.key_mode !== 'clear'"
                variant="text"
                data-testid="key-clear"
                @click="d.key_mode = 'clear'"
                >Clear</UiButton
              >
              <UiButton v-if="d.key_mode === 'clear'" variant="text" @click="d.key_mode = 'keep'"
                >Undo</UiButton
              >
            </template>
          </div>
          <p class="note">
            Run the LLM server on your home network or tailnet only; it has no password by default.
          </p>
          <p v-if="testText" class="note" role="status" data-testid="test-result">{{ testText }}</p>
          <p v-if="backlogText" class="note" role="status" data-testid="backlog">
            {{ backlogText }}
          </p>
          <UiButton
            v-if="backlogText && ai.endpoint?.enabled"
            variant="text"
            :disabled="processing"
            data-testid="process-now"
            @click="processNow"
            >Process now</UiButton
          >
        </template>
        <template v-if="showForm" #actions>
          <UiButton variant="tonal" :disabled="testing" data-testid="test" @click="testConnection"
            >Test connection</UiButton
          >
        </template>
      </UiCard>
      <UiSaveBar
        v-if="showForm"
        :dirty="form.dirty.value"
        :saving="form.saving.value"
        :error="form.error.value"
        @save="form.save"
        @discard="form.discard"
      />

      <UiCard data-testid="mail-mode-card">
        <template #header>Mail processing</template>
        <UiRadioGroup
          :model-value="mailMode"
          legend="Who reads incoming mail first"
          :options="mailModeOptions"
          @update:model-value="mailModeSave.set($event as MailMode)"
        />
        <template v-if="llmFirst">
          <UiTextField
            v-model="catchupForm.draft.value.catchup_days"
            label="Catch up on mail from the last (days)"
            name="catchup_days"
            type="number"
            :error="catchupErr"
            :supporting="catchupHint"
          />
        </template>
      </UiCard>
      <UiSaveBar
        v-if="llmFirst"
        :dirty="catchupForm.dirty.value"
        :saving="catchupForm.saving.value"
        :error="catchupForm.error.value"
        @save="catchupForm.save"
        @discard="catchupForm.discard"
      />

      <UiCard data-testid="stages-card">
        <template #header>Email decisions</template>
        <p v-if="!ai.endpoint || !enabled" class="note">
          These stages wait for the endpoint: connect and enable it above first.
        </p>
        <UiList>
          <UiListItem
            v-for="p in ai.pipeline"
            :key="p.decision_point"
            :headline="stageName(p.decision_point)"
            :supporting="
              llmFirst
                ? `The AI decides; its answer is used above ${pct(thresholds[p.decision_point] ?? p.llm_threshold)}`
                : `Ask the LLM when rules are unsure, above ${pct(thresholds[p.decision_point] ?? p.llm_threshold)}`
            "
          >
            <template #trailing>
              <div class="stage">
                <input
                  v-if="!llmFirst"
                  type="range"
                  min="0.5"
                  max="0.99"
                  step="0.01"
                  :value="rulesThresholds[p.decision_point] ?? p.rules_threshold"
                  :aria-label="`${stageName(p.decision_point)} rules threshold`"
                  data-testid="rules-threshold"
                  @input="
                    rulesThresholds[p.decision_point] = Number(
                      ($event.target as HTMLInputElement).value,
                    )
                  "
                  @change="releaseRulesThreshold(p, $event)"
                />
                <input
                  type="range"
                  min="0.5"
                  max="0.95"
                  step="0.05"
                  :value="thresholds[p.decision_point] ?? p.llm_threshold"
                  :disabled="!llmOn(p)"
                  :aria-label="`${stageName(p.decision_point)} threshold`"
                  @input="
                    thresholds[p.decision_point] = Number(($event.target as HTMLInputElement).value)
                  "
                  @change="releaseThreshold(p, $event)"
                />
                <UiSwitch
                  :model-value="llmOn(p)"
                  :label="`${stageName(p.decision_point)} LLM`"
                  @update:model-value="toggleStage(p, $event)"
                />
              </div>
            </template>
          </UiListItem>
        </UiList>
      </UiCard>

      <UiCard data-testid="extract-card">
        <template #header>Extraction</template>
        <UiSegmented
          v-if="!llmFirst"
          :model-value="mode"
          label="Pull events, bills and tasks out of mail with the LLM"
          :options="modeOptions"
          @update:model-value="modeSave.set($event as typeof mode)"
        />
        <label v-if="!llmFirst && mode === 'fallback'" class="cutoff">
          <span class="note"
            >Ask the AI when no rule extractor is sure above {{ pct(cutoff) }}</span
          >
          <input
            v-model.number="cutoff"
            type="range"
            min="0.5"
            max="0.99"
            step="0.01"
            data-testid="cutoff"
            aria-label="Extraction fallback cutoff"
            @change="releaseCutoff"
          />
        </label>
        <p v-if="llmFirst" class="note">
          The AI reads every message and pulls out events, bills and tasks.
        </p>
        <UiTextField
          v-model="maxForm.draft.value.max_input_chars"
          label="Longest text sent to the model (characters)"
          name="max_input_chars"
          type="number"
          :error="maxErr"
        />
      </UiCard>
      <UiSaveBar
        :dirty="maxForm.dirty.value"
        :saving="maxForm.saving.value"
        :error="maxForm.error.value"
        @save="maxForm.save"
        @discard="maxForm.discard"
      />

      <UiCard data-testid="activity-card">
        <template #header>Activity (last 7 days)</template>
        <UiEmptyState v-if="noActivity" icon="ai" title="No AI activity yet." />
        <template v-else>
          <UiList>
            <UiListItem
              v-for="c in callRows"
              :key="c.purpose"
              :headline="c.purpose"
              :supporting="`${c.total} calls, ${c.share}% ok, p50 ${ms(c.p50_ms)}, p95 ${ms(c.p95_ms)}`"
            />
          </UiList>
          <div v-for="s in stageRows" :key="s.decision_point" class="stat">
            <span class="stat__name">{{ stageName(s.decision_point) }}</span>
            <div
              class="bar"
              role="img"
              :aria-label="`${stageName(s.decision_point)}: rules ${s.rules}, LLM ${s.llm}, person ${s.human}`"
            >
              <span class="seg seg--rules" :style="{ width: s.rp + '%' }" />
              <span class="seg seg--llm" :style="{ width: s.lp + '%' }" />
              <span class="seg seg--human" :style="{ width: s.hp + '%' }" />
            </div>
            <span class="note">Rules {{ s.rules }} · LLM {{ s.llm }} · Person {{ s.human }}</span>
          </div>
        </template>
        <template #actions>
          <UiButton variant="text" icon="refresh" :disabled="statsBusy" @click="refreshStats"
            >Refresh</UiButton
          >
        </template>
      </UiCard>
    </template>
  </SettingsPage>
</template>

<style scoped>
.note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.row {
  display: flex;
  align-items: flex-end;
  gap: var(--ui-space-2);
}
.row > :first-child {
  flex: 1;
}
.key {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ui-space-2);
}
.stage {
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
}
.stage input[type='range'] {
  width: 7rem;
  accent-color: var(--ui-primary);
}
.cutoff {
  display: grid;
  gap: var(--ui-space-1);
}
.cutoff input[type='range'] {
  accent-color: var(--ui-primary);
}
.stat {
  display: grid;
  gap: var(--ui-space-1);
  padding: var(--ui-space-2) 0;
}
.stat__name {
  font: var(--ui-type-label-lg);
  color: var(--ui-on-surface);
}
.bar {
  display: flex;
  height: 0.5rem;
  overflow: hidden;
  border-radius: var(--ui-radius-full);
  background: var(--ui-surface-container-highest);
}
.seg--rules {
  background: var(--ui-primary);
}
.seg--llm {
  background: var(--ui-warning);
}
.seg--human {
  background: var(--ui-success);
}
</style>
