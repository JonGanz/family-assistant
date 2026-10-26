<script setup lang="ts">
import PageHeader from '../components/PageHeader.vue'
import { computed, onMounted, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { TaskView } from '../api/client'
import ErrorBanner from '../components/ErrorBanner.vue'
import StepList from '../components/tasks/StepList.vue'
import { calendarDisplayLabel, type CalendarDisplay, type PriorityFlag } from '../lib/tasks'
import { useHouseholdStore } from '../stores/household'
import { useListsStore } from '../stores/lists'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import { useTasksStore } from '../stores/tasks'
import { fromInstant, toInstant } from './parts/dateInput'
import { describeError } from './shared'

const route = useRoute()
const router = useRouter()
const session = useSessionStore()
const household = useHouseholdStore()
const members = useMembersStore()
const lists = useListsStore()
const tasks = useTasksStore()

const id = computed(() => String(route.params.id))
const task = ref<TaskView | null>(null)
const loadError = ref('')
const error = ref('')
const saved = ref(false)

const form = reactive({
  title: '',
  notes: '',
  date: '',
  time: '',
  assignees: [] as string[],
  priority: 'normal' as PriorityFlag,
  display: 'hidden' as CalendarDisplay,
  steps: [] as string[],
})
const newStep = ref('')

function fill(t: TaskView) {
  task.value = t
  const due = fromInstant(t.due_at, household.tz)
  form.title = t.title
  form.notes = t.notes ?? ''
  form.date = due.date
  form.time = due.time
  form.assignees = [...t.assignees]
  form.priority = t.priority_flag
  form.display = t.calendar_display
  form.steps = t.steps.map((s) => s.text)
}

async function load() {
  loadError.value = ''
  try {
    fill(await tasks.get(id.value))
  } catch (e) {
    loadError.value = describeError(e, 'Could not load the task.')
  }
}

onMounted(async () => {
  void members.load()
  void lists.load()
  await load()
})

const list = computed(() => lists.items.find((l) => l.id === task.value?.list))
/** Adults edit everything; a child edits their own one-off tasks on lists that allow it (D-045). */
const canEdit = computed(() => {
  const t = task.value
  if (!t) return false
  if (session.isAdult) return true
  const me = session.member?.id
  return (
    t.origin === 'manual' && list.value?.child_can_add === 'own' && !!me && t.assignees.includes(me)
  )
})
const people = computed(() =>
  session.isAdult ? members.items : members.items.filter((m) => m.id === session.member?.id),
)
const sourceLabel = computed(
  () =>
    ({
      manual: 'Added by hand',
      system: 'Repeating task',
      weather: 'From the weather',
      email: 'From an email',
    })[task.value?.origin ?? 'manual'],
)

function moveStep(i: number, d: number) {
  const j = i + d
  if (j < 0 || j >= form.steps.length) return
  const s = form.steps
  ;[s[i], s[j]] = [s[j]!, s[i]!]
}
function addStep() {
  const t = newStep.value.trim()
  if (t) form.steps.push(t)
  newStep.value = ''
}
function toggleAssignee(mid: string) {
  form.assignees = form.assignees.includes(mid)
    ? form.assignees.filter((x) => x !== mid)
    : [...form.assignees, mid]
}

async function save() {
  const t = task.value
  if (!t) return
  error.value = ''
  saved.value = false
  try {
    const v = await tasks.save(t, {
      title: form.title.trim(),
      notes: form.notes.trim() || null,
      due_at: toInstant(form.date, form.time, household.tz),
      assignees: form.assignees,
      priority_flag: form.priority,
      calendar_display: form.display,
      steps: form.steps,
    })
    if (v) {
      fill(v)
      saved.value = true
    }
  } catch (e) {
    error.value = describeError(e, 'Save failed.')
  }
}

async function tick(stepId: string, done: boolean) {
  error.value = ''
  try {
    await tasks.tick(id.value, stepId, done)
    fill(await tasks.get(id.value))
  } catch (e) {
    error.value = describeError(e, 'Could not update the step.')
  }
}

async function remove() {
  if (!task.value || !confirm(`Delete “${task.value.title}”?`)) return
  try {
    await tasks.remove(task.value.id)
    await router.replace({ name: 'tasks' })
  } catch (e) {
    error.value = describeError(e, 'Could not delete the task.')
  }
}

const stepModels = computed(
  () => task.value?.steps.map((s) => ({ id: s.id, text: s.text, done: !!s.done_at })) ?? [],
)
</script>

<template>
  <section>
    <PageHeader v-if="!task" title="Task" back="/tasks" />
    <ErrorBanner v-if="loadError" :message="loadError" />
    <template v-if="task">
      <PageHeader :title="task.title" back="/tasks">
        {{ sourceLabel }} · {{ task.list_name }} · {{ task.status }}
        <template v-if="task.definition && session.isAdult">
          ·
          <RouterLink
            :to="{ path: '/settings/tasks', query: { tab: 'definitions', def: task.definition } }"
            >Edit the schedule</RouterLink
          >
        </template>
      </PageHeader>
      <p v-if="task.assignee_away" class="muted">An assignee is away on the due date.</p>

      <form class="form" @submit.prevent="save">
        <label
          >Title <input v-model="form.title" name="title" required :disabled="!canEdit"
        /></label>
        <label>
          Notes
          <textarea v-model="form.notes" name="notes" rows="3" :disabled="!canEdit" />
        </label>
        <label
          >Due date <input v-model="form.date" name="due_date" type="date" :disabled="!canEdit"
        /></label>
        <label v-if="form.date">
          Due time <input v-model="form.time" name="due_time" type="time" :disabled="!canEdit" />
        </label>
        <fieldset>
          <legend>Assigned to</legend>
          <label v-for="m in people" :key="m.id" class="field-inline">
            <input
              type="checkbox"
              :checked="form.assignees.includes(m.id)"
              :disabled="!canEdit || !session.isAdult"
              @change="toggleAssignee(m.id)"
            />
            {{ m.display_name }}
          </label>
          <p v-if="form.assignees.length === 0" class="muted">Anyone in the household.</p>
        </fieldset>
        <label>
          Priority
          <select v-model="form.priority" name="priority" :disabled="!canEdit">
            <option value="normal">Normal</option>
            <option value="high">High</option>
          </select>
        </label>
        <label>
          On the calendar
          <select v-model="form.display" name="calendar_display" :disabled="!canEdit">
            <option
              v-for="d in ['hidden', 'until_done', 'always'] as CalendarDisplay[]"
              :key="d"
              :value="d"
            >
              {{ calendarDisplayLabel(d) }}
            </option>
          </select>
        </label>

        <fieldset>
          <legend>Steps</legend>
          <ol v-if="canEdit" class="edit-steps">
            <li v-for="(_s, i) in form.steps" :key="i">
              <input v-model="form.steps[i]" :aria-label="`Step ${i + 1}`" />
              <button
                type="button"
                :disabled="i === 0"
                aria-label="Move up"
                @click="moveStep(i, -1)"
              >
                ↑
              </button>
              <button
                type="button"
                :disabled="i === form.steps.length - 1"
                aria-label="Move down"
                @click="moveStep(i, 1)"
              >
                ↓
              </button>
              <button type="button" aria-label="Remove step" @click="form.steps.splice(i, 1)">
                ✕
              </button>
            </li>
          </ol>
          <div v-if="canEdit" class="add-step">
            <input
              v-model="newStep"
              name="new_step"
              placeholder="Add a step"
              @keydown.enter.prevent="addStep"
            />
            <button type="button" @click="addStep">Add step</button>
          </div>
          <StepList
            v-if="stepModels.length"
            :steps="stepModels"
            :disabled="task.status !== 'open'"
            @tick="tick"
          />
          <p v-else class="muted">No steps.</p>
        </fieldset>

        <ErrorBanner v-if="tasks.conflict" conflict @reload="load" />
        <ErrorBanner v-if="error" :message="error" />
        <p v-if="saved" class="text-ok" role="status">Saved.</p>
        <div v-if="canEdit" class="row">
          <button type="submit" class="btn-primary">Save</button>
          <button type="button" class="btn-danger" @click="remove">Delete</button>
        </div>
      </form>
    </template>
  </section>
</template>

<style scoped>
.add-step,
.edit-steps li {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}
.edit-steps {
  padding: 0;
  list-style: none;
  display: grid;
  gap: var(--space-1);
}
.edit-steps input {
  flex: 1;
}
form > label:not(.field-inline) {
  display: grid;
  gap: var(--space-1);
}
fieldset {
  display: grid;
  gap: var(--space-2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}
.text-ok {
  color: var(--color-success);
}
</style>
