<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import type { WeatherRuleView } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { ApiError } from '../../api/http'
import { useDefinitionsStore } from '../../stores/definitions'
import { useRulesStore } from '../../stores/rules'
import { describeError } from '../shared'
import { useRuleValidation } from './useRuleValidation'

const props = defineProps<{ rule: WeatherRuleView | null }>()
const emit = defineEmits<{ done: [] }>()

const rules = useRulesStore()
const defs = useDefinitionsStore()
const name = ref(props.rule?.name ?? '')
const condition = ref(props.rule?.condition ?? '')
const message = ref(props.rule?.message ?? '')
const severity = ref(props.rule?.severity ?? 'info')
const action = ref(props.rule?.action ?? 'alert')
const taskMode = ref<'create' | 'suggest'>(props.rule?.task_mode ?? 'create')
const taskDef = ref(
  ((props.rule?.task_definition ?? null) as { definition?: string } | null)?.definition ?? '',
)
const saveError = ref('')
const conditionEl = ref<HTMLInputElement | null>(null)

onMounted(() => void defs.load())

const { result, failure, pending } = useRuleValidation(condition, message)

const FIELDS = [
  'temp_c',
  'feels_c',
  'precip_prob',
  'precip_mm',
  'snow_cm',
  'gust_kmh',
  'wind_kmh',
  'uv',
  'code',
]
const WINDOWS = ['next 12h', 'next 24h', 'today', 'tonight', 'tomorrow']
const AGGS = ['min', 'max', 'sum', 'avg', 'any']

const error = computed(() => result.value?.error ?? null)
/** Marker line: spaces up to the error position, then a caret. */
const marker = computed(() =>
  error.value ? `${' '.repeat(Math.min(error.value.position, condition.value.length))}^` : '',
)
const preview = computed(() => (result.value?.ok ? result.value.preview : null))

function insert(text: string) {
  const el = conditionEl.value
  const v = condition.value
  const at = el?.selectionStart ?? v.length
  const end = el?.selectionEnd ?? at
  condition.value = v.slice(0, at) + text + v.slice(end)
  el?.focus()
}

/** Task options only apply to the `task` action; `task_definition` is `{definition: id}`. */
function taskFields() {
  if (action.value !== 'task') return {}
  return {
    task_mode: taskMode.value,
    task_definition: (taskDef.value ? { definition: taskDef.value } : null) as Record<
      string,
      never
    > | null,
  }
}

async function save() {
  saveError.value = ''
  if (action.value === 'task' && !taskDef.value) {
    saveError.value = 'Pick the chore or task this rule should create.'
    return
  }
  try {
    if (props.rule) {
      const ok = await rules.patch(props.rule, {
        name: name.value,
        condition: condition.value,
        message: message.value,
        severity: severity.value,
        action: action.value,
        ...taskFields(),
      })
      if (!ok) return
    } else {
      await rules.create({
        name: name.value,
        condition: condition.value,
        message: message.value,
        severity: severity.value,
        action: action.value,
        ...taskFields(),
      })
    }
    emit('done')
  } catch (e) {
    if (e instanceof ApiError && e.status === 422) {
      const pos = e.problem.position
      saveError.value = `${e.problem.message ?? e.message}${typeof pos === 'number' ? ` (at character ${pos + 1})` : ''}`
    } else saveError.value = describeError(e, 'Save failed.')
  }
}
</script>

<template>
  <form class="form editor" @submit.prevent="save">
    <label>Name <input v-model="name" name="rule_name" required /></label>
    <label>
      Condition
      <input
        ref="conditionEl"
        v-model="condition"
        class="mono"
        name="condition"
        spellcheck="false"
        autocomplete="off"
        required
        :aria-invalid="!!error"
        aria-describedby="rule-feedback"
      />
    </label>
    <div class="chips" role="group" aria-label="Insert into the condition">
      <button v-for="a in AGGS" :key="a" type="button" @click="insert(`${a}(`)">{{ a }}</button>
      <button v-for="f in FIELDS" :key="f" type="button" @click="insert(f)">{{ f }}</button>
      <button v-for="w in WINDOWS" :key="w" type="button" @click="insert(w)">{{ w }}</button>
    </div>
    <div id="rule-feedback" aria-live="polite">
      <pre v-if="error" class="text-danger mono" data-testid="rule-error"
        >{{ condition }}
{{ marker }}
{{ error.message }}</pre>
      <p v-else-if="pending" class="muted">Checking…</p>
      <p v-else-if="failure" class="text-danger">{{ failure }}</p>
      <p v-else-if="preview" class="text-success" data-testid="rule-preview">
        With the latest forecast this rule
        <strong>{{ preview.matches ? 'would fire' : 'would not fire' }}</strong>
        <template v-if="preview.value != null"> (value {{ preview.value }})</template>
        <template v-if="preview.when"> in the {{ preview.when }}</template
        >.
      </p>
      <p v-else-if="result?.ok" class="text-success">The rule is valid.</p>
    </div>
    <label>
      Message
      <input v-model="message" name="message" required />
      <span class="muted">Use {value}, {when} and {field}.</span>
    </label>
    <label>
      Severity
      <select v-model="severity" name="severity">
        <option value="info">Info</option>
        <option value="warning">Warning</option>
        <option value="severe">Severe</option>
      </select>
    </label>
    <label>
      Action
      <select v-model="action" name="action">
        <option value="alert">Show an advisory</option>
        <option value="task">Create a task</option>
      </select>
    </label>
    <template v-if="action === 'task'">
      <label>
        Task to create
        <select v-model="taskDef" name="task_definition" required>
          <option value="" disabled>Pick a chore or schedule</option>
          <option v-for="d in defs.items" :key="d.id" :value="d.id">{{ d.title }}</option>
        </select>
        <span class="muted">
          Pick from the chores in Settings → Tasks. It is made once per day the rule fires.
        </span>
      </label>
      <label>
        What happens
        <select v-model="taskMode" name="task_mode">
          <option value="create">Create the task right away</option>
          <option value="suggest">Suggest it (an adult accepts or dismisses)</option>
        </select>
      </label>
    </template>
    <ErrorBanner v-if="rules.conflict" conflict @reload="rules.load()" />
    <ErrorBanner v-if="saveError" :message="saveError" />
    <div class="row">
      <button type="submit" :disabled="!!error">Save</button>
      <button type="button" @click="emit('done')">Cancel</button>
    </div>
  </form>
</template>

<style scoped>
.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.chips,
.row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}
.chips button {
  font-family: ui-monospace, monospace;
  font-size: var(--font-size-sm);
}
pre {
  margin: 0;
  white-space: pre-wrap;
}
.editor {
  max-width: 40rem;
}
</style>
