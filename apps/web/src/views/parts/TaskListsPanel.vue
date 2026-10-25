<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import type { TaskListView } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { calendarDisplayLabel } from '../../lib/tasks'
import { useListsStore } from '../../stores/lists'
import { describeError } from '../shared'
import {
  emptyListForm,
  listDefaults,
  listFormFrom,
  listInput,
  listPatch,
  type ListForm,
  type ListKind,
} from './listForm'
import './settings.css'

const lists = useListsStore()
const editing = ref<string | null>(null)
const form = reactive<ListForm>(emptyListForm(0))
const error = ref('')

onMounted(() => lists.load())

function open(l: TaskListView | null) {
  Object.assign(form, l ? listFormFrom(l) : emptyListForm(lists.items.length))
  editing.value = l ? l.id : 'new'
  error.value = ''
}
function setKind(k: ListKind) {
  form.kind = k
  Object.assign(form, listDefaults(k))
}
async function save() {
  error.value = ''
  try {
    if (editing.value === 'new') await lists.create(listInput(form))
    else {
      const l = lists.items.find((x) => x.id === editing.value)
      if (l && !(await lists.patch(l, listPatch(form)))) return
    }
    editing.value = null
  } catch (e) {
    error.value = describeError(e, 'Save failed.')
  }
}
async function remove(l: TaskListView) {
  if (!confirm(`Delete the list “${l.name}”?`)) return
  try {
    await lists.remove(l.id)
  } catch (e) {
    error.value = describeError(e, 'Could not delete the list.')
  }
}
</script>

<template>
  <div>
    <ErrorBanner v-if="lists.error" :message="lists.error" />
    <ErrorBanner v-if="error && editing === null" :message="error" />
    <ul class="settings-list">
      <li v-for="l in lists.items" :key="l.id" :data-testid="`list-${l.name}`">
        <template v-if="editing !== l.id">
          <strong>{{ l.name }}</strong> <span class="badge">{{ l.kind }}</span>
          <p class="settings-note">
            Children add: {{ l.child_can_add === 'own' ? 'their own' : 'no' }} · Calendar:
            {{ calendarDisplayLabel(l.calendar_default) }} ·
            {{ l.completion_policy === 'adult_only' ? 'adults complete' : 'assignee completes' }}
          </p>
          <button type="button" @click="open(l)">Edit</button>
          <button type="button" @click="remove(l)">Delete</button>
        </template>
      </li>
    </ul>

    <form v-if="editing" class="settings-form" @submit.prevent="save">
      <h2>{{ editing === 'new' ? 'New list' : 'Edit list' }}</h2>
      <label>Name <input v-model="form.name" name="name" required /></label>
      <label>
        Kind (sets the defaults below)
        <select
          :value="form.kind"
          name="kind"
          @change="setKind(($event.target as HTMLSelectElement).value as ListKind)"
        >
          <option value="chores">Chores</option>
          <option value="homework">Homework</option>
          <option value="general">General</option>
        </select>
      </label>
      <label>
        Who can complete a task
        <select v-model="form.completion_policy" name="completion_policy">
          <option value="assignee">The person it's for (or anyone, if unassigned)</option>
          <option value="adult_only">Adults only</option>
        </select>
      </label>
      <label>
        Children can add tasks
        <select v-model="form.child_can_add" name="child_can_add">
          <option value="none">No</option>
          <option value="own">Yes, for themselves</option>
        </select>
      </label>
      <label class="inline">
        <input v-model="form.dashboard_add" type="checkbox" name="dashboard_add" />
        Allow adding from the wall dashboard
      </label>
      <label>
        On the calendar by default
        <select v-model="form.calendar_default" name="calendar_default">
          <option value="hidden">Hidden</option>
          <option value="until_done">Until done</option>
          <option value="always">Key date</option>
        </select>
      </label>
      <fieldset>
        <legend>A claimed task is held</legend>
        <label class="inline">
          <input v-model="form.claim" type="radio" value="end_of_day" name="claim" /> until the end
          of the day
        </label>
        <label class="inline">
          <input v-model="form.claim" type="radio" value="minutes" name="claim" /> for a number of
          minutes
        </label>
        <label v-if="form.claim === 'minutes'">
          Minutes
          <input
            v-model.number="form.claimMinutes"
            name="claim_minutes"
            type="number"
            min="1"
            max="10080"
          />
        </label>
      </fieldset>
      <label class="inline">
        <input v-model="form.visible_on_dashboard" type="checkbox" name="visible_on_dashboard" />
        Show on the dashboard
      </label>
      <label class="inline">
        <input v-model="form.useColor" type="checkbox" name="use_color" /> Colour
        <input v-if="form.useColor" v-model="form.color" type="color" name="color" />
      </label>
      <label>Order <input v-model.number="form.sort" name="sort" type="number" /></label>
      <ErrorBanner v-if="lists.conflict" conflict @reload="lists.load()" />
      <ErrorBanner v-if="error" :message="error" />
      <div class="row">
        <button type="submit">Save</button>
        <button type="button" @click="editing = null">Cancel</button>
      </div>
    </form>
    <button v-else type="button" @click="open(null)">Add a list</button>
  </div>
</template>

<style scoped>
.row {
  display: flex;
  gap: var(--space-2);
}
</style>
