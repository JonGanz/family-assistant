<script setup lang="ts">
import { computed, reactive, ref, toRef } from 'vue'
import type { TaskDefinitionView } from '../../api/client'
import { ApiError } from '../../api/http'
import ErrorBanner from '../../components/ErrorBanner.vue'
import RruleBuilder from '../../components/tasks/RruleBuilder.vue'
import { useDefinitionsStore } from '../../stores/definitions'
import { useListsStore } from '../../stores/lists'
import { useMembersStore } from '../../stores/members'
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
import './settings.css'

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
  <form class="settings-form editor" @submit.prevent="save">
    <label>Title <input v-model="form.title" name="title" required /></label>
    <label>
      List
      <select v-model="form.list" name="list" required>
        <option v-for="l in lists.items" :key="l.id" :value="l.id">{{ l.name }}</option>
      </select>
    </label>
    <label>Notes <textarea v-model="form.notes" name="notes" rows="2" /></label>
    <label>
      Steps (one per line)
      <textarea v-model="form.steps" name="steps" rows="3" />
    </label>

    <fieldset>
      <legend>Schedule</legend>
      <label class="inline">
        <input
          type="radio"
          name="schedule_kind"
          :checked="form.scheduleKind === 'after_done'"
          @change="setKind('after_done')"
        />
        Every N days after it's done
      </label>
      <label class="inline">
        <input
          type="radio"
          name="schedule_kind"
          :checked="form.scheduleKind === 'rrule'"
          @change="setKind('rrule')"
        />
        Repeats
      </label>
      <label v-if="form.scheduleKind === 'after_done'">
        Days
        <input
          v-model.number="form.intervalDays"
          name="interval_days"
          type="number"
          min="1"
          max="3650"
        />
      </label>
      <template v-else>
        <RruleBuilder v-model="form.rrule" />
        <p v-if="rruleError" class="settings-error" data-testid="rrule-error">
          {{ rruleError.message }}
        </p>
      </template>
      <label
        >Start date <input v-model="form.startDate" name="start_date" type="date" required
      /></label>
      <label
        >Due time (optional) <input v-model="form.dueTime" name="due_time" type="time"
      /></label>
      <label>
        Show it this many days before it's due
        <input v-model.number="form.leadDays" name="lead_days" type="number" min="0" max="60" />
      </label>
      <label>
        If it's missed
        <select v-model="form.onMissed" name="on_missed" @change="onMissedTouched = true">
          <option
            v-for="k in ['roll_over', 'auto_skip', 'pile_up'] as OnMissed[]"
            :key="k"
            :value="k"
          >
            {{ { roll_over: 'Roll over', auto_skip: 'Skip it', pile_up: 'Pile up' }[k] }}
          </option>
        </select>
        <span class="settings-note" data-testid="on-missed-help">{{
          ON_MISSED_HELP[form.onMissed]
        }}</span>
      </label>
      <div v-if="result?.next_dates.length" data-testid="next-dates">
        <span class="settings-note">Next dates:</span>
        <ul class="dates">
          <li v-for="d in result.next_dates" :key="d">{{ d }}</li>
        </ul>
      </div>
    </fieldset>

    <fieldset>
      <legend>Who</legend>
      <label>
        Strategy
        <select
          :value="form.strategy"
          name="strategy"
          @change="setStrategy(($event.target as HTMLSelectElement).value as Strategy)"
        >
          <option value="fixed">Fixed person</option>
          <option value="rotation">Rotation</option>
          <option value="pool">Pool</option>
        </select>
        <span class="settings-note">{{ STRATEGY_HELP[form.strategy] }}</span>
      </label>
      <label v-for="m in members.items" :key="m.id" class="inline">
        <input
          type="checkbox"
          :checked="form.candidates.includes(m.id)"
          @change="toggleCandidate(m.id)"
        />
        {{ m.display_name }}
      </label>
      <ol v-if="form.strategy === 'rotation' && form.candidates.length > 1" class="order">
        <li v-for="(c, i) in form.candidates" :key="c">
          {{ nameOf(c) }}
          <button type="button" :disabled="i === 0" aria-label="Move up" @click="move(i, -1)">
            ↑
          </button>
          <button
            type="button"
            :disabled="i === form.candidates.length - 1"
            aria-label="Move down"
            @click="move(i, 1)"
          >
            ↓
          </button>
        </li>
      </ol>
      <template v-if="form.strategy === 'rotation'">
        <label>
          The turn moves on when
          <select v-model="form.advanceOn" name="advance_on">
            <option value="any_completion">anyone does it</option>
            <option value="assignee_completion">only the person whose turn it is does it</option>
          </select>
        </label>
        <label class="inline">
          <input v-model="form.skipAdvances" type="checkbox" name="skip_advances" />
          Skipping it also moves the turn on
        </label>
      </template>
    </fieldset>

    <label>
      Priority
      <select v-model="form.priority" name="priority">
        <option value="normal">Normal</option>
        <option value="high">High</option>
      </select>
    </label>

    <fieldset>
      <legend>Season (optional)</legend>
      <label class="inline">
        <input v-model="form.seasonOn" type="checkbox" name="season_on" /> Only in part of the year
      </label>
      <template v-if="form.seasonOn">
        <label
          >From (month-day)
          <input
            v-model="form.seasonFrom"
            name="season_from"
            placeholder="04-01"
            pattern="\d{2}-\d{2}"
        /></label>
        <label
          >Until (month-day)
          <input
            v-model="form.seasonUntil"
            name="season_until"
            placeholder="10-31"
            pattern="\d{2}-\d{2}"
        /></label>
      </template>
      <label>
        Only when the weather says so (optional)
        <input
          v-model="form.condition"
          name="season_condition"
          class="mono"
          spellcheck="false"
          autocomplete="off"
          placeholder="sum(precip_mm, today) < 5"
          :aria-invalid="!!conditionError"
        />
      </label>
      <div aria-live="polite">
        <pre v-if="conditionError" class="settings-error mono" data-testid="condition-error"
          >{{ form.condition }}
{{ marker }}
{{ conditionError.message }}</pre>
        <p v-else-if="pending" class="settings-note">Checking…</p>
        <p v-else-if="failure" class="settings-error">{{ failure }}</p>
        <p v-else-if="form.condition.trim() && result?.ok" class="settings-ok">
          The condition is valid.
        </p>
      </div>
    </fieldset>

    <label class="inline"
      ><input v-model="form.paused" type="checkbox" name="paused" /> Paused</label
    >

    <ul v-if="problems.length" class="settings-note">
      <li v-for="p in problems" :key="p">{{ p }}</li>
    </ul>
    <ErrorBanner v-if="defs.conflict" conflict @reload="defs.load()" />
    <ErrorBanner v-if="saveError" :message="saveError" />
    <div class="row">
      <button type="submit" :disabled="!!blocked">Save</button>
      <button type="button" @click="emit('done')">Cancel</button>
    </div>
  </form>
</template>

<style scoped>
.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.row {
  display: flex;
  gap: var(--space-2);
}
.dates {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  list-style: none;
  padding: 0;
  margin: 0;
}
.order {
  padding-left: var(--space-5);
}
pre {
  margin: 0;
  white-space: pre-wrap;
}
.editor {
  max-width: 40rem;
}
</style>
