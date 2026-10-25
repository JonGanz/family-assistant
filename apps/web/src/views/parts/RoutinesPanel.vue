<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { api, type CalendarView, type RoutineView } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { useMembersStore } from '../../stores/members'
import { useRoutinesStore } from '../../stores/routines'
import { describeError } from '../shared'
import {
  emptyRoutineForm,
  routineFormFrom,
  routineInput,
  routinePatch,
  routineProblems,
  type RoutineForm,
} from './routineForm'
import './settings.css'

const routines = useRoutinesStore()
const members = useMembersStore()
const editing = ref<string | null>(null)
const form = reactive<RoutineForm>(emptyRoutineForm())
const calendars = ref<CalendarView[]>([])
const error = ref('')
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
async function remove(r: RoutineView) {
  if (!confirm(`Delete the routine “${r.name}”?`)) return
  try {
    await routines.remove(r.id)
  } catch (e) {
    error.value = describeError(e, 'Could not delete the routine.')
  }
}
</script>

<template>
  <div>
    <ErrorBanner v-if="routines.error" :message="routines.error" />
    <ErrorBanner v-if="error && editing === null" :message="error" />
    <ul class="settings-list">
      <li v-for="r in routines.items" :key="r.id" :data-testid="`routine-${r.name}`">
        <strong>{{ r.name }}</strong>
        <p class="settings-note">
          {{ r.members.map(nameOf).join(', ') }} · {{ r.starts_at.slice(0, 5) }}–{{
            r.done_by.slice(0, 5)
          }}
          · {{ r.steps.length }} steps · {{ r.day_filter.replace('_', ' ') }}
        </p>
        <button type="button" @click="open(r)">Edit</button>
        <button type="button" @click="remove(r)">Delete</button>
      </li>
    </ul>

    <form v-if="editing" class="settings-form" @submit.prevent="save">
      <h2>{{ editing === 'new' ? 'New routine' : 'Edit routine' }}</h2>
      <label>Name <input v-model="form.name" name="name" required /></label>
      <fieldset>
        <legend>Who</legend>
        <label v-for="m in members.items" :key="m.id" class="inline">
          <input
            type="checkbox"
            :checked="form.members.includes(m.id)"
            @change="toggleMember(m.id)"
          />
          {{ m.display_name }}
        </label>
      </fieldset>
      <label>Starts <input v-model="form.startsAt" name="starts_at" type="time" required /></label>
      <label>Done by <input v-model="form.doneBy" name="done_by" type="time" required /></label>
      <fieldset>
        <legend>Steps</legend>
        <div v-for="(s, i) in form.steps" :key="i" class="step">
          <input v-model="s.text" :aria-label="`Step ${i + 1}`" placeholder="Brush teeth" />
          <input v-model="s.target" type="time" :aria-label="`Target time for step ${i + 1}`" />
          <button type="button" :disabled="i === 0" aria-label="Move up" @click="move(i, -1)">
            ↑
          </button>
          <button
            type="button"
            :disabled="i === form.steps.length - 1"
            aria-label="Move down"
            @click="move(i, 1)"
          >
            ↓
          </button>
          <button type="button" aria-label="Remove step" @click="form.steps.splice(i, 1)">✕</button>
        </div>
        <button type="button" @click="form.steps.push({ text: '', target: '' })">Add step</button>
      </fieldset>
      <label>
        Days
        <select v-model="form.dayFilter" name="day_filter">
          <option value="daily">Every day</option>
          <option value="weekdays">Weekdays</option>
          <option value="weekends">Weekends</option>
          <option value="school_days">School days</option>
        </select>
      </label>
      <template v-if="form.dayFilter === 'school_days'">
        <label>
          School calendar
          <select v-model="form.schoolCalendar" name="school_calendar">
            <option value="">None (acts like weekdays)</option>
            <option v-for="c in calendars" :key="c.id" :value="c.id">{{ c.name }}</option>
          </select>
        </label>
        <label>
          A day is off when an all-day event contains (comma separated)
          <input v-model="form.keywords" name="keywords" />
        </label>
      </template>
      <label class="inline">
        <input v-model="form.showOnDashboard" type="checkbox" name="show_on_dashboard" />
        Show on the dashboard
      </label>
      <ul v-if="problems.length" class="settings-note">
        <li v-for="p in problems" :key="p">{{ p }}</li>
      </ul>
      <ErrorBanner v-if="routines.conflict" conflict @reload="routines.load()" />
      <ErrorBanner v-if="error" :message="error" />
      <div class="row">
        <button type="submit" :disabled="problems.length > 0">Save</button>
        <button type="button" @click="editing = null">Cancel</button>
      </div>
    </form>
    <button v-else type="button" @click="open(null)">Add a routine</button>
  </div>
</template>

<style scoped>
.step,
.row {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}
.step input:first-child {
  flex: 1;
}
</style>
