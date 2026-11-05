<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import type { WeatherRuleView } from '../../api/client'
import { ApiError } from '../../api/http'
import { useDefinitionsStore } from '../../stores/definitions'
import { useRulesStore } from '../../stores/rules'
import { UiButton, UiCard, UiChip, UiSelect, UiTextField } from '../../ui'
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
const conditionBox = ref<HTMLElement | null>(null)

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

const severityOptions = [
  { value: 'info', label: 'Info' },
  { value: 'warning', label: 'Warning' },
  { value: 'severe', label: 'Severe' },
]
const actionOptions = [
  { value: 'alert', label: 'Show an advisory' },
  { value: 'task', label: 'Create a task' },
]
const modeOptions = [
  { value: 'create', label: 'Create the task right away' },
  { value: 'suggest', label: 'Suggest it (an adult accepts or dismisses)' },
]
const defOptions = computed(() => [
  { value: '', label: 'Pick a chore or schedule', disabled: true },
  ...defs.items.map((d) => ({ value: d.id, label: d.title })),
])
const error = computed(() => result.value?.error ?? null)
/** Marker line: spaces up to the error position, then a caret. */
const marker = computed(() =>
  error.value ? `${' '.repeat(Math.min(error.value.position, condition.value.length))}^` : '',
)
const preview = computed(() => (result.value?.ok ? result.value.preview : null))

function insert(text: string) {
  const el = conditionBox.value?.querySelector('input') ?? null
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
  <form class="editor" @submit.prevent="save">
    <UiTextField v-model="name" label="Name" name="rule_name" required />
    <div ref="conditionBox">
      <UiTextField
        v-model="condition"
        label="Condition"
        class="rule-mono"
        name="condition"
        spellcheck="false"
        autocomplete="off"
        :aria-invalid="!!error"
        aria-describedby="rule-feedback"
      />
    </div>
    <div class="chips" role="group" aria-label="Insert into the condition">
      <UiChip v-for="a in AGGS" :key="a" @click="insert(`${a}(`)">{{ a }}</UiChip>
      <UiChip v-for="f in FIELDS" :key="f" @click="insert(f)">{{ f }}</UiChip>
      <UiChip v-for="w in WINDOWS" :key="w" @click="insert(w)">{{ w }}</UiChip>
    </div>
    <div id="rule-feedback" aria-live="polite">
      <pre v-if="error" class="fb fb--error" data-testid="rule-error"
        >{{ condition }}
{{ marker }}
{{ error.message }}</pre>
      <p v-else-if="pending" class="fb">Checking…</p>
      <p v-else-if="failure" class="fb fb--error">{{ failure }}</p>
      <p v-else-if="preview" class="fb" data-testid="rule-preview">
        With the latest forecast this rule
        <strong>{{ preview.matches ? 'would fire' : 'would not fire' }}</strong>
        <template v-if="preview.value != null"> (value {{ preview.value }})</template>
        <template v-if="preview.when"> in the {{ preview.when }}</template
        >.
      </p>
      <p v-else-if="result?.ok" class="fb">The rule is valid.</p>
    </div>
    <UiTextField
      v-model="message"
      label="Message"
      name="message"
      required
      supporting="Use {value}, {when} and {field}."
    />
    <UiSelect
      :model-value="severity"
      label="Severity"
      :options="severityOptions"
      @update:model-value="severity = $event as typeof severity"
    />
    <UiSelect
      :model-value="action"
      label="Action"
      :options="actionOptions"
      @update:model-value="action = $event as typeof action"
    />
    <template v-if="action === 'task'">
      <UiSelect
        v-model="taskDef"
        label="Task to create"
        :options="defOptions"
        supporting="Pick from the chores in Settings → Tasks. It is made once per day the rule fires."
      />
      <UiSelect
        :model-value="taskMode"
        label="What happens"
        :options="modeOptions"
        @update:model-value="taskMode = $event as 'create' | 'suggest'"
      />
    </template>
    <UiCard v-if="rules.conflict" variant="outlined" role="alert">
      <p>This page changed elsewhere. Reload to see the latest.</p>
      <template #actions
        ><UiButton variant="tonal" @click="rules.load()">Reload</UiButton></template
      >
    </UiCard>
    <UiCard v-if="saveError" variant="outlined" role="alert">{{ saveError }}</UiCard>
    <div class="row">
      <UiButton variant="text" @click="emit('done')">Cancel</UiButton>
      <UiButton type="submit" :disabled="!!error || !condition.trim()">Save</UiButton>
    </div>
  </form>
</template>

<style scoped>
.editor {
  display: grid;
  gap: var(--ui-space-3);
}
.chips,
.row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.row {
  justify-content: flex-end;
}
.fb {
  margin: 0;
  font: var(--ui-type-body-sm);
}
.fb--error {
  color: var(--ui-error);
}
pre.fb {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  white-space: pre-wrap;
}
.editor :deep(.rule-mono) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
</style>
