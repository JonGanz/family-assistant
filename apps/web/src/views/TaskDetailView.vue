<script setup lang="ts">
// Task detail and editor (M4). One draft and one Save bar for the page, because the API takes a
// single versioned PATCH; the priority flag and the calendar display apply at once with Undo.
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { TaskPatch, TaskView } from '../api/client'
import StepEditor from '../components/tasks/StepEditor.vue'
import StepList from '../components/tasks/StepList.vue'
import { calendarDisplayLabel, type CalendarDisplay, type PriorityFlag } from '../lib/tasks'
import { useHouseholdStore } from '../stores/household'
import { useListsStore } from '../stores/lists'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import { useTasksStore } from '../stores/tasks'
import {
  UiButton,
  UiCard,
  UiChip,
  UiDialog,
  UiErrorState,
  UiIconButton,
  UiPage,
  UiSaveBar,
  UiSegmented,
  UiSkeleton,
  UiSwitch,
  UiTextField,
  UiTopAppBar,
  useFormDraft,
  useInstantSave,
  useSnackbar,
} from '../ui'
import { fromInstant, toInstant } from './parts/dateInput'
import { describeError } from './shared'

const route = useRoute()
const router = useRouter()
const session = useSessionStore()
const household = useHouseholdStore()
const members = useMembersStore()
const lists = useListsStore()
const tasks = useTasksStore()
const snackbar = useSnackbar()

const id = computed(() => String(route.params.id))
const task = ref<TaskView | null>(null)
const loadError = ref('')
const confirmDelete = ref(false)

interface Draft {
  title: string
  notes: string
  date: string
  time: string
  assignees: string[]
  steps: string[]
}

function fromTask(): Draft {
  const t = task.value
  const due = fromInstant(t?.due_at, household.tz)
  return {
    title: t?.title ?? '',
    notes: t?.notes ?? '',
    date: due.date,
    time: due.time,
    assignees: t ? [...t.assignees] : [],
    steps: t ? t.steps.map((s) => s.text) : [],
  }
}

const SAVE_CONFLICT = 'This task was changed elsewhere. Reload to see the latest.'

async function patch(p: TaskPatch): Promise<TaskView> {
  const t = task.value
  if (!t) throw new Error('The task is not loaded.')
  let v: TaskView | null
  try {
    v = await tasks.save(t, p)
  } catch (e) {
    throw new Error(describeError(e, 'Save failed.'), { cause: e })
  }
  if (!v) throw new Error(SAVE_CONFLICT)
  task.value = v
  return v
}

const form = useFormDraft(fromTask, async (d) => {
  await patch({
    title: d.title.trim(),
    notes: d.notes.trim() || null,
    due_at: toInstant(d.date, d.time, household.tz),
    assignees: d.assignees,
    steps: d.steps,
  })
})
const draft = form.draft

// Instant controls (D-096): the flag and the display mode.
const priority = ref<PriorityFlag>('normal')
const display = ref<CalendarDisplay>('hidden')
const savePriority = useInstantSave(
  priority,
  async (v) => void (await patch({ priority_flag: v })),
  { label: 'Priority saved' },
)
const saveDisplay = useInstantSave(
  display,
  async (v) => void (await patch({ calendar_display: v })),
  { label: 'Calendar display saved' },
)

function fill(t: TaskView) {
  task.value = t
  priority.value = t.priority_flag
  display.value = t.calendar_display
}

async function load() {
  loadError.value = ''
  try {
    fill(await tasks.get(id.value))
    tasks.conflict = false
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
const displayOptions = (['hidden', 'until_done', 'always'] as CalendarDisplay[]).map((d) => ({
  value: d,
  label: calendarDisplayLabel(d),
}))
const stepModels = computed(
  () => task.value?.steps.map((s) => ({ id: s.id, text: s.text, done: !!s.done_at })) ?? [],
)

function toggleAssignee(mid: string) {
  draft.value.assignees = draft.value.assignees.includes(mid)
    ? draft.value.assignees.filter((x) => x !== mid)
    : [...draft.value.assignees, mid]
}

async function tick(stepId: string, done: boolean) {
  try {
    await tasks.tick(id.value, stepId, done)
    fill(await tasks.get(id.value))
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not update the step.') })
  }
}

async function remove() {
  confirmDelete.value = false
  if (!task.value) return
  try {
    await tasks.remove(task.value.id)
    await router.replace({ name: 'tasks' })
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not delete the task.') })
  }
}
const back = () => void router.push({ name: 'tasks' })
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar :title="task?.title ?? 'Task'" back @back="back">
        <template v-if="task && canEdit" #actions>
          <UiIconButton icon="delete" label="Delete task" @click="confirmDelete = true" />
        </template>
      </UiTopAppBar>
    </template>

    <UiErrorState v-if="loadError" :message="loadError" @retry="load" />
    <UiSkeleton v-else-if="!task" :lines="6" />

    <template v-else>
      <p class="meta">
        {{ sourceLabel }} · {{ task.list_name }} · {{ task.status }}
        <template v-if="task.definition && session.isAdult">
          ·
          <RouterLink
            :to="{ path: '/settings/tasks', query: { tab: 'definitions', def: task.definition } }"
            >Edit the schedule</RouterLink
          >
        </template>
      </p>
      <p v-if="task.assignee_away" class="meta">An assignee is away on the due date.</p>
      <div v-if="tasks.conflict" class="conflict" role="alert">
        <span>{{ SAVE_CONFLICT }}</span>
        <UiButton variant="tonal" icon="refresh" @click="load">Reload</UiButton>
      </div>

      <UiCard variant="filled">
        <div class="title-field">
          <UiTextField
            v-model="draft.title"
            label="Title"
            name="title"
            required
            :disabled="!canEdit"
          />
        </div>
      </UiCard>

      <UiCard variant="filled">
        <template #header><h2 class="card-title">Notes</h2></template>
        <UiTextField
          v-model="draft.notes"
          label="Notes"
          name="notes"
          multiline
          :disabled="!canEdit"
        />
      </UiCard>

      <UiCard variant="filled">
        <template #header><h2 class="card-title">Due and display</h2></template>
        <div class="stack">
          <UiTextField
            v-model="draft.date"
            label="Due date"
            name="due_date"
            type="date"
            :disabled="!canEdit"
          />
          <UiTextField
            v-if="draft.date"
            v-model="draft.time"
            label="Due time"
            name="due_time"
            type="time"
            :disabled="!canEdit"
          />
          <UiSwitch
            label="High priority"
            :model-value="priority === 'high'"
            :disabled="!canEdit"
            @update:model-value="savePriority.set($event ? 'high' : 'normal')"
          />
          <UiSegmented
            label="On the calendar"
            :model-value="display"
            :options="displayOptions"
            @update:model-value="canEdit && saveDisplay.set($event as CalendarDisplay)"
          />
        </div>
      </UiCard>

      <UiCard variant="filled">
        <template #header><h2 class="card-title">Steps</h2></template>
        <div class="stack">
          <StepEditor v-if="canEdit" v-model="draft.steps" />
          <StepList
            v-if="stepModels.length"
            :steps="stepModels"
            :disabled="task.status !== 'open'"
            @tick="tick"
          />
          <p v-else-if="!canEdit" class="meta">No steps.</p>
        </div>
      </UiCard>

      <UiCard variant="filled">
        <template #header><h2 class="card-title">Assigned to</h2></template>
        <div class="chips" role="group" aria-label="Assigned to">
          <UiChip
            v-for="m in people"
            :key="m.id"
            variant="filter"
            :selected="draft.assignees.includes(m.id)"
            :disabled="!canEdit || !session.isAdult"
            @click="toggleAssignee(m.id)"
          >
            {{ m.display_name }}
          </UiChip>
        </div>
        <p v-if="draft.assignees.length === 0" class="meta">Anyone in the household.</p>
      </UiCard>

      <UiSaveBar
        :dirty="form.dirty.value"
        :saving="form.saving.value"
        :error="form.error.value"
        @save="form.save"
        @discard="form.discard"
      />
    </template>

    <UiDialog v-model="confirmDelete" :title="`Delete “${task?.title ?? ''}”?`">
      This removes the task for everyone.
      <template #actions>
        <UiButton variant="text" @click="confirmDelete = false">Cancel</UiButton>
        <UiButton @click="remove">Delete</UiButton>
      </template>
    </UiDialog>
  </UiPage>
</template>

<style scoped>
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.card-title {
  margin: 0;
  font: var(--ui-type-title-md);
}
.stack {
  display: grid;
  gap: var(--ui-space-4);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.conflict {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ui-space-3);
  padding: var(--ui-space-3) var(--ui-space-4);
  border-radius: var(--ui-radius-md);
  background: var(--ui-error-container);
  color: var(--ui-on-error-container);
}
.title-field :deep(.ui-field__input) {
  font: var(--ui-type-headline-sm);
}
</style>
