<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { api, type CalendarView, type RoutineView } from '../../api/client'
import ResponsiveItems from '../../app/ResponsiveItems.vue'
import { useMembersStore } from '../../stores/members'
import { useRoutinesStore } from '../../stores/routines'
import {
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiIconButton,
  UiSelect,
  UiSkeleton,
  UiTextField,
} from '../../ui'
import { describeError } from '../shared'
import ConfirmDialog from './settings-mail/ConfirmDialog.vue'
import PageFab from './settings-mail/PageFab.vue'
import PageNotices from './settings-mail/PageNotices.vue'
import SheetForm from './settings-mail/SheetForm.vue'
import { useConfirm } from './settings-mail/useConfirm'
import {
  emptyRoutineForm,
  routineFormFrom,
  routineInput,
  routinePatch,
  routineProblems,
  type RoutineForm,
} from './routineForm'

const routines = useRoutinesStore()
const members = useMembersStore()
const editing = ref<string | null>(null)
const form = reactive<RoutineForm>(emptyRoutineForm())
const calendars = ref<CalendarView[]>([])
const error = ref('')
const confirm = useConfirm()
const dayOptions = [
  { value: 'daily', label: 'Every day' },
  { value: 'weekdays', label: 'Weekdays' },
  { value: 'weekends', label: 'Weekends' },
  { value: 'school_days', label: 'School days' },
]
const calendarOptions = computed(() => [
  { value: '', label: 'None (acts like weekdays)' },
  ...calendars.value.map((c) => ({ value: c.id, label: c.name })),
])
const problems = computed(() => routineProblems(form))

onMounted(async () => {
  void routines.load()
  void members.load()
  try {
    calendars.value = await api.calendars()
  } catch {
    calendars.value = []
  }
})

const nameOf = (id: string) => members.items.find((m) => m.id === id)?.display_name ?? '?'

function open(r: RoutineView | null) {
  Object.assign(form, r ? routineFormFrom(r) : emptyRoutineForm())
  editing.value = r ? r.id : 'new'
  error.value = ''
}
function toggleMember(id: string) {
  form.members = form.members.includes(id)
    ? form.members.filter((x) => x !== id)
    : [...form.members, id]
}
function move(i: number, d: number) {
  const j = i + d
  if (j < 0 || j >= form.steps.length) return
  const s = form.steps
  ;[s[i], s[j]] = [s[j]!, s[i]!]
}
async function save() {
  error.value = ''
  try {
    if (editing.value === 'new') await routines.create(routineInput(form))
    else {
      const r = routines.items.find((x) => x.id === editing.value)
      if (r && !(await routines.patch(r, routinePatch(form)))) return
    }
    editing.value = null
  } catch (e) {
    error.value = describeError(e, 'Save failed.')
  }
}
const remove = (r: RoutineView) =>
  confirm.ask(`Delete the routine “${r.name}”?`, 'Its steps and history go with it.', async () => {
    try {
      await routines.remove(r.id)
    } catch (e) {
      error.value = describeError(e, 'Could not delete the routine.')
    }
  })
</script>

<template>
  <div class="panel">
    <PageNotices
      :load-error="routines.error"
      :conflict="routines.conflict && !editing"
      :error="editing ? '' : error"
      @reload="routines.load()"
    />
    <UiSkeleton v-if="!routines.loaded && !routines.error" :lines="3" />
    <UiEmptyState
      v-else-if="routines.loaded && !routines.items.length"
      icon="routines"
      title="No routines yet."
    />
    <ResponsiveItems :items="routines.items">
      <template #item="{ item: r }">
        <div class="item" :data-testid="`routine-${r.name}`">
          <strong class="item__name">{{ r.name }}</strong>
          <p class="meta">
            {{ r.members.map(nameOf).join(', ') }} · {{ r.starts_at.slice(0, 5) }}–{{
              r.done_by.slice(0, 5)
            }}
            · {{ r.steps.length }} steps · {{ r.day_filter.replace('_', ' ') }}
          </p>
          <div class="item__row">
            <UiButton variant="text" @click="open(r)">Edit</UiButton>
            <UiButton variant="text" @click="remove(r)">Delete</UiButton>
          </div>
        </div>
      </template>
    </ResponsiveItems>

    <PageFab label="Add a routine" @click="open(null)" />
    <SheetForm
      :model-value="!!editing"
      :title="editing === 'new' ? 'New routine' : 'Edit routine'"
      :save-disabled="problems.length > 0"
      @update:model-value="editing = null"
      @save="save"
    >
      <UiTextField v-model="form.name" label="Name" name="name" required />
      <fieldset>
        <legend>Who</legend>
        <UiCheckbox
          v-for="m in members.items"
          :key="m.id"
          :model-value="form.members.includes(m.id)"
          :label="m.display_name"
          @update:model-value="toggleMember(m.id)"
        />
      </fieldset>
      <UiTextField v-model="form.startsAt" label="Starts" name="starts_at" type="time" required />
      <UiTextField v-model="form.doneBy" label="Done by" name="done_by" type="time" required />
      <fieldset>
        <legend>Steps</legend>
        <div v-for="(s, i) in form.steps" :key="i" class="step">
          <UiTextField v-model="s.text" :label="`Step ${i + 1}`" placeholder="Brush teeth" />
          <UiTextField v-model="s.target" type="time" :label="`Target time for step ${i + 1}`" />
          <div class="step__btns">
            <UiIconButton
              icon="collapse"
              label="Move up"
              :disabled="i === 0"
              @click="move(i, -1)"
            />
            <UiIconButton
              icon="expand"
              label="Move down"
              :disabled="i === form.steps.length - 1"
              @click="move(i, 1)"
            />
            <UiIconButton icon="close" label="Remove step" @click="form.steps.splice(i, 1)" />
          </div>
        </div>
        <div>
          <UiButton variant="tonal" icon="add" @click="form.steps.push({ text: '', target: '' })"
            >Add step</UiButton
          >
        </div>
      </fieldset>
      <UiSelect
        :model-value="form.dayFilter ?? 'daily'"
        label="Days"
        :options="dayOptions"
        @update:model-value="form.dayFilter = $event as typeof form.dayFilter"
      />
      <template v-if="form.dayFilter === 'school_days'">
        <UiSelect
          v-model="form.schoolCalendar"
          label="School calendar"
          :options="calendarOptions"
        />
        <UiTextField
          v-model="form.keywords"
          label="A day is off when an all-day event contains (comma separated)"
          name="keywords"
        />
      </template>
      <UiCheckbox v-model="form.showOnDashboard" label="Show on the dashboard" />
      <ul v-if="problems.length" class="meta">
        <li v-for="p in problems" :key="p">{{ p }}</li>
      </ul>
      <p v-if="error" role="alert">{{ error }}</p>
    </SheetForm>
    <ConfirmDialog
      :pending="confirm.pending.value"
      @accept="confirm.accept"
      @cancel="confirm.cancel"
    />
  </div>
</template>

<style scoped>
.panel,
.item {
  display: grid;
  gap: var(--ui-space-3);
}
.item {
  gap: var(--ui-space-2);
}
.item__row {
  display: flex;
  gap: var(--ui-space-2);
}
.item__name {
  font: var(--ui-type-title-md);
}
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
fieldset {
  display: grid;
  gap: var(--ui-space-3);
  border: 0;
  padding: 0;
  margin: 0;
}
legend {
  padding: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-label-lg);
}
.step {
  display: grid;
  gap: var(--ui-space-2);
  padding-bottom: var(--ui-space-3);
  border-bottom: 1px solid var(--ui-outline-variant);
}
.step__btns {
  display: flex;
  gap: var(--ui-space-1);
}
</style>
