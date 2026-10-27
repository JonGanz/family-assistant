<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import type { TaskDefinitionView } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { scheduleSummary } from '../../lib/tasks'
import { dayKey } from '../../lib/time'
import { useDefinitionsStore } from '../../stores/definitions'
import { useHouseholdStore } from '../../stores/household'
import { useListsStore } from '../../stores/lists'
import { useMembersStore } from '../../stores/members'
import { describeError } from '../shared'
import DefinitionEditor from './DefinitionEditor.vue'
import { formFromView, toPatch } from './definitionForm'

const route = useRoute()
const defs = useDefinitionsStore()
const lists = useListsStore()
const members = useMembersStore()
const household = useHouseholdStore()
const editing = ref<string | null>(typeof route.query.def === 'string' ? route.query.def : null)
const error = ref('')
const today = computed(() => dayKey(new Date(), household.tz))

onMounted(() => {
  void defs.load()
  void lists.load()
  void members.load()
})

const listName = (id: string) => lists.items.find((l) => l.id === id)?.name ?? ''
async function act(fn: () => Promise<unknown>) {
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e, 'Could not save.')
  }
}
const togglePause = (d: TaskDefinitionView) =>
  act(() => defs.patch(d, { ...toPatch(formFromView(d)), paused: !d.paused }))
const remove = (d: TaskDefinitionView) => {
  if (confirm(`Delete “${d.title}”? Open tasks already made from it stay.`))
    void act(() => defs.remove(d.id))
}
</script>

<template>
  <div>
    <ErrorBanner v-if="defs.error" :message="defs.error" />
    <ErrorBanner v-if="defs.conflict && editing === null" conflict @reload="defs.load()" />
    <ErrorBanner v-if="error" :message="error" />
    <p v-if="lists.loaded && lists.items.length === 0" class="muted">
      Create a list first (Lists tab).
    </p>
    <ul class="card-list">
      <li v-for="d in defs.items" :key="d.id" :data-testid="`def-${d.title}`">
        <template v-if="editing !== d.id">
          <strong>{{ d.title }}</strong>
          <span v-if="d.paused" class="badge needs_reauth">paused</span>
          <span v-if="d.condition_ok === false" class="badge">waiting for weather</span>
          <p class="muted">
            {{ listName(d.list) }} · {{ scheduleSummary(d.schedule) }} · {{ d.strategy }}
            <template v-if="d.next_due_at"> · next {{ d.next_due_at.slice(0, 10) }}</template>
          </p>
          <div class="actions">
            <button type="button" @click="editing = d.id">Edit</button>
            <button type="button" @click="togglePause(d)">
              {{ d.paused ? 'Resume' : 'Pause' }}
            </button>
            <button type="button" @click="remove(d)">Delete</button>
          </div>
        </template>
        <DefinitionEditor v-else :definition="d" :today="today" @done="editing = null" />
      </li>
    </ul>
    <DefinitionEditor
      v-if="editing === 'new'"
      :definition="null"
      :today="today"
      @done="editing = null"
    />
    <button v-else-if="lists.items.length" type="button" @click="editing = 'new'">
      Add a chore or schedule
    </button>
  </div>
</template>

<style scoped>
.actions {
  display: flex;
  gap: var(--space-2);
}
</style>
