<script setup lang="ts">
import { computed, reactive, ref, toRef } from 'vue'
import type { TaskDefinitionView } from '../../api/client'
import { ApiError } from '../../api/http'
import RruleBuilder from '../../components/tasks/RruleBuilder.vue'
import { useDefinitionsStore } from '../../stores/definitions'
import { useListsStore } from '../../stores/lists'
import { useMembersStore } from '../../stores/members'
import {
  UiButton,
  UiCard,
  UiCheckbox,
  UiIconButton,
  UiRadioGroup,
  UiSelect,
  UiTextField,
} from '../../ui'
import { describeError } from '../shared'
import {
  emptyForm,
  defaultOnMissed,
  formFromView,
  formProblems,
  ON_MISSED_HELP,
  STRATEGY_HELP,
  toInput,
  toPatch,
  type OnMissed,
  type ScheduleKind,
  type Strategy,
} from './definitionForm'
import { useDefinitionValidation } from './useDefinitionValidation'

const props = defineProps<{ definition: TaskDefinitionView | null; today: string }>()
const emit = defineEmits<{ done: [] }>()

const defs = useDefinitionsStore()
const lists = useListsStore()
const members = useMembersStore()

const form = reactive(
  props.definition
    ? formFromView(props.definition)
    : emptyForm(lists.items[0]?.id ?? '', props.today),
)
const onMissedTouched = ref(!!props.definition)
const saveError = ref('')

const kindOptions = [
  { value: 'after_done', label: "Every N days after it's done" },
  { value: 'rrule', label: 'Repeats' },
]
const missedOptions = (['roll_over', 'auto_skip', 'pile_up'] as OnMissed[]).map((k) => ({
  value: k,
  label: { roll_over: 'Roll over', auto_skip: 'Skip it', pile_up: 'Pile up' }[k],
}))
const strategyOptions = [
  { value: 'fixed', label: 'Fixed person' },
  { value: 'rotation', label: 'Rotation' },
  { value: 'pool', label: 'Pool' },
]
const advanceOptions = [
  { value: 'any_completion', label: 'anyone does it' },
  { value: 'assignee_completion', label: 'only the person whose turn it is does it' },
]
const priorityOptions = [
  { value: 'normal', label: 'Normal' },
  { value: 'high', label: 'High' },
]
const listOptions = computed(() => lists.items.map((l) => ({ value: l.id, label: l.name })))
const num = (v: string) => Number(v)

const rruleRef = computed(() => (form.scheduleKind === 'rrule' ? form.rrule : null))
const { result, failure, pending } = useDefinitionValidation(
  rruleRef,
  toRef(form, 'condition'),
  toRef(form, 'startDate'),
)

const errorFor = (field: string) => result.value?.errors.find((e) => e.field === field) ?? null
const conditionError = computed(() => errorFor('season_condition'))
const rruleError = computed(() => errorFor('rrule'))
const marker = computed(() =>
  conditionError.value
    ? `${' '.repeat(Math.min(conditionError.value.position ?? 0, form.condition.length))}^`
    : '',
)
const problems = computed(() => formProblems(form))
const blocked = computed(() => problems.value.length > 0 || (result.value && !result.value.ok))

function setKind(k: ScheduleKind) {
  form.scheduleKind = k
  if (!onMissedTouched.value) form.onMissed = defaultOnMissed(k)
}
function setStrategy(s: Strategy) {
  form.strategy = s
  if (s === 'fixed' && form.candidates.length > 1) form.candidates = form.candidates.slice(0, 1)
}
function toggleCandidate(id: string) {
  if (form.candidates.includes(id)) form.candidates = form.candidates.filter((x) => x !== id)
  else form.candidates = form.strategy === 'fixed' ? [id] : [...form.candidates, id]
}
function move(i: number, d: number) {
  const j = i + d
  if (j < 0 || j >= form.candidates.length) return
  const c = form.candidates
  ;[c[i], c[j]] = [c[j]!, c[i]!]
}
const nameOf = (id: string) => members.items.find((m) => m.id === id)?.display_name ?? '?'

async function save() {
  saveError.value = ''
  try {
    if (props.definition) {
      if (!(await defs.patch(props.definition, toPatch(form)))) return
    } else {
      await defs.create(toInput(form))
    }
    emit('done')
  } catch (e) {
    if (e instanceof ApiError && e.status === 422) {
      const pos = e.problem.position
      const field = typeof e.problem.field === 'string' ? `${e.problem.field}: ` : ''
      saveError.value = `${field}${e.problem.message ?? e.message}${typeof pos === 'number' ? ` (at character ${pos + 1})` : ''}`
    } else saveError.value = describeError(e, 'Save failed.')
  }
}
</script>

<template>
  <form class="editor" @submit.prevent="save">
    <UiTextField v-model="form.title" label="Title" name="title" required />
    <UiSelect v-model="form.list" label="List" :options="listOptions" />
    <UiTextField v-model="form.notes" label="Notes" name="notes" multiline />
    <UiTextField v-model="form.steps" label="Steps (one per line)" name="steps" multiline />

    <fieldset>
      <legend>Schedule</legend>
      <UiRadioGroup
        :model-value="form.scheduleKind"
        legend="Schedule kind"
        :options="kindOptions"
        @update:model-value="setKind($event as ScheduleKind)"
      />
      <UiTextField
        v-if="form.scheduleKind === 'after_done'"
        :model-value="String(form.intervalDays)"
        label="Days"
        name="interval_days"
        type="number"
        min="1"
        max="3650"
        @update:model-value="form.intervalDays = num($event)"
      />
      <template v-else>
        <RruleBuilder v-model="form.rrule" />
        <p v-if="rruleError" class="err" data-testid="rrule-error">{{ rruleError.message }}</p>
      </template>
      <UiTextField
        v-model="form.startDate"
        label="Start date"
        name="start_date"
        type="date"
        required
      />
      <UiTextField v-model="form.dueTime" label="Due time (optional)" name="due_time" type="time" />
      <UiTextField
        :model-value="String(form.leadDays)"
        label="Show it this many days before it's due"
        name="lead_days"
        type="number"
        min="0"
        max="60"
        @update:model-value="form.leadDays = num($event)"
      />
      <UiSelect
        :model-value="form.onMissed"
        label="If it's missed"
        :options="missedOptions"
        :supporting="ON_MISSED_HELP[form.onMissed]"
        data-testid="on-missed"
        @update:model-value="((form.onMissed = $event as OnMissed), (onMissedTouched = true))"
      />
      <div v-if="result?.next_dates.length" data-testid="next-dates">
        <span class="muted">Next dates:</span>
        <ul class="dates">
          <li v-for="d in result.next_dates" :key="d">{{ d }}</li>
        </ul>
      </div>
    </fieldset>

    <fieldset>
      <legend>Who</legend>
      <UiSelect
        :model-value="form.strategy"
        label="Strategy"
        :options="strategyOptions"
        :supporting="STRATEGY_HELP[form.strategy]"
        @update:model-value="setStrategy($event as Strategy)"
      />
      <UiCheckbox
        v-for="m in members.items"
        :key="m.id"
        :model-value="form.candidates.includes(m.id)"
        :label="m.display_name"
        @update:model-value="toggleCandidate(m.id)"
      />
      <ol v-if="form.strategy === 'rotation' && form.candidates.length > 1" class="order">
        <li v-for="(c, i) in form.candidates" :key="c">
          <span>{{ nameOf(c) }}</span>
          <UiIconButton icon="collapse" label="Move up" :disabled="i === 0" @click="move(i, -1)" />
          <UiIconButton
            icon="expand"
            label="Move down"
            :disabled="i === form.candidates.length - 1"
            @click="move(i, 1)"
          />
        </li>
      </ol>
      <template v-if="form.strategy === 'rotation'">
        <UiSelect
          :model-value="form.advanceOn ?? 'any_completion'"
          label="The turn moves on when"
          :options="advanceOptions"
          @update:model-value="form.advanceOn = $event as typeof form.advanceOn"
        />
        <UiCheckbox v-model="form.skipAdvances" label="Skipping it also moves the turn on" />
      </template>
    </fieldset>

    <UiSelect
      :model-value="form.priority ?? 'normal'"
      label="Priority"
      :options="priorityOptions"
      @update:model-value="form.priority = $event as typeof form.priority"
    />

    <fieldset>
      <legend>Season (optional)</legend>
      <UiCheckbox v-model="form.seasonOn" label="Only in part of the year" />
      <template v-if="form.seasonOn">
        <UiTextField
          v-model="form.seasonFrom"
          label="From (month-day)"
          name="season_from"
          placeholder="04-01"
          pattern="\d{2}-\d{2}"
        />
        <UiTextField
          v-model="form.seasonUntil"
          label="Until (month-day)"
          name="season_until"
          placeholder="10-31"
          pattern="\d{2}-\d{2}"
        />
      </template>
      <div class="cond">
        <UiTextField
          v-model="form.condition"
          label="Only when the weather says so (optional)"
          class="def-mono"
          name="season_condition"
          spellcheck="false"
          autocomplete="off"
          placeholder="sum(precip_mm, today) < 5"
          :aria-invalid="!!conditionError"
        />
      </div>
      <div aria-live="polite">
        <pre v-if="conditionError" class="err" data-testid="condition-error"
          >{{ form.condition }}
{{ marker }}
{{ conditionError.message }}</pre>
        <p v-else-if="pending" class="muted">Checking…</p>
        <p v-else-if="failure" class="err">{{ failure }}</p>
        <p v-else-if="form.condition.trim() && result?.ok" class="muted">The condition is valid.</p>
      </div>
    </fieldset>

    <UiCheckbox v-model="form.paused" label="Paused" />

    <ul v-if="problems.length" class="muted">
      <li v-for="p in problems" :key="p">{{ p }}</li>
    </ul>
    <UiCard v-if="defs.conflict" variant="outlined" role="alert">
      <p>This page changed elsewhere. Reload to see the latest.</p>
      <template #actions><UiButton variant="tonal" @click="defs.load()">Reload</UiButton></template>
    </UiCard>
    <UiCard v-if="saveError" variant="outlined" role="alert">{{ saveError }}</UiCard>
    <div class="row">
      <UiButton variant="text" @click="emit('done')">Cancel</UiButton>
      <UiButton type="submit" :disabled="!!blocked">Save</UiButton>
    </div>
  </form>
</template>

<style scoped>
.editor,
fieldset {
  display: grid;
  gap: var(--ui-space-3);
}
fieldset {
  border: 0;
  padding: 0;
  margin: 0;
}
legend {
  padding: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-label-lg);
}
.row {
  display: flex;
  justify-content: flex-end;
  gap: var(--ui-space-2);
}
.muted {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.err {
  margin: 0;
  color: var(--ui-error);
  font: var(--ui-type-body-sm);
  white-space: pre-wrap;
}
pre.err {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.cond :deep(.def-mono) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.dates {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
  list-style: none;
  padding: 0;
  margin: 0;
}
.order {
  display: grid;
  gap: var(--ui-space-1);
  margin: 0;
  padding-left: var(--ui-space-5);
}
.order li {
  display: list-item;
}
</style>
