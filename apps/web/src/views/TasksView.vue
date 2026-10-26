<script setup lang="ts">
import PageHeader from '../components/PageHeader.vue'
import StateBlock from '../components/StateBlock.vue'
import ErrorBanner from '../components/ErrorBanner.vue'
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import type { TaskView } from '../api/client'
import TaskItem from '../components/tasks/TaskItem.vue'
import UndoToast from '../components/tasks/UndoToast.vue'
import { useClock } from '../composables/useClock'
import { useHouseholdStore } from '../stores/household'
import { useListsStore } from '../stores/lists'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import { addableLists, memberRefs, toTaskItem, type Viewer } from '../stores/taskMap'
import { useTasksStore, type TaskTab } from '../stores/tasks'
import { describeError } from './shared'
import TaskAddForm from './parts/TaskAddForm.vue'

type Tab = 'now' | 'mine' | 'lists' | 'suggested'

const session = useSessionStore()
const household = useHouseholdStore()
const members = useMembersStore()
const lists = useListsStore()
const tasks = useTasksStore()
const router = useRouter()
const now = useClock(30_000)

const tab = ref<Tab>('now')
const adding = ref(false)
const error = ref('')

const viewer = computed<Viewer | null>(() => {
  const m = session.member
  return m ? { id: m.id, role: m.role === 'adult' ? 'adult' : 'child' } : null
})
const tabs = computed<Tab[]>(() =>
  session.isAdult ? ['now', 'mine', 'lists', 'suggested'] : ['now', 'mine', 'lists'],
)
const refs = computed(() => memberRefs(members.items))
const canAdd = computed(() => addableLists(lists.items, viewer.value).length > 0)

const dataTab = computed<TaskTab>(() => (tab.value === 'lists' ? 'all' : tab.value))
function load() {
  void tasks.load(dataTab.value)
}
onMounted(() => {
  void members.load()
  void lists.load()
  load()
})
watch(tab, load)

const current = computed(() => tasks.tabs[dataTab.value] ?? [])
const isMine = (t: TaskView) => !!viewer.value && t.assignees.includes(viewer.value.id)
const nowMine = computed(() => current.value.filter(isMine))
const nowHousehold = computed(() => current.value.filter((t) => !isMine(t)))
const byList = computed(() =>
  lists.items
    .map((l) => ({ list: l, items: current.value.filter((t) => t.list === l.id) }))
    .filter((g) => g.items.length > 0),
)

const model = (t: TaskView) => toTaskItem(t, refs.value)
const open = (id: string) => router.push({ name: 'task-detail', params: { id } })

async function act(fn: () => Promise<unknown>) {
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e, 'That did not work.')
  }
}
const complete = (t: TaskView) => act(() => tasks.complete(t))
const claimedByMe = (t: TaskView) => !!viewer.value && t.claimed_by === viewer.value.id
const undo = () => act(() => tasks.undoLast())
</script>

<template>
  <section>
    <PageHeader title="Tasks">
      <template #actions>
        <button v-if="canAdd && !adding" type="button" class="btn-primary" @click="adding = true">
          + Task
        </button>
      </template>
    </PageHeader>
    <TaskAddForm
      v-if="adding"
      :lists="lists.items"
      :members="members.items"
      :viewer="viewer"
      :tz="household.tz"
      @done="adding = false"
    />

    <div class="tabs" role="tablist">
      <button
        v-for="t in tabs"
        :key="t"
        type="button"
        role="tab"
        :aria-selected="tab === t"
        :aria-pressed="tab === t"
        @click="tab = t"
      >
        {{ { now: 'Now', mine: 'Mine', lists: 'Lists', suggested: 'Suggested' }[t] }}
      </button>
    </div>

    <StateBlock :error="tasks.error" />
    <ErrorBanner v-if="error" :message="error" />

    <template v-if="tab === 'now'">
      <h2>Mine</h2>
      <p v-if="nowMine.length === 0" class="muted">Nothing for you right now.</p>
      <div class="rows">
        <TaskItem
          v-for="t in nowMine"
          :key="t.id"
          :task="model(t)"
          :now="now"
          :tz="household.tz"
          @complete="complete(t)"
          @open="open"
          @claim="act(() => tasks.claim(t.id))"
        >
          <template v-if="claimedByMe(t)" #claim>
            <button type="button" class="btn-sm" @click="act(() => tasks.release(t.id))">
              Release
            </button>
          </template>
        </TaskItem>
      </div>
      <h2>Household</h2>
      <p v-if="nowHousehold.length === 0" class="muted">Nothing else is due.</p>
      <div class="rows">
        <TaskItem
          v-for="t in nowHousehold"
          :key="t.id"
          :task="model(t)"
          :now="now"
          :tz="household.tz"
          @complete="complete(t)"
          @open="open"
          @claim="act(() => tasks.claim(t.id))"
        />
      </div>
    </template>

    <template v-else-if="tab === 'mine'">
      <p v-if="current.length === 0" class="muted">You have no open tasks.</p>
      <div class="rows">
        <TaskItem
          v-for="t in current"
          :key="t.id"
          :task="model(t)"
          :now="now"
          :tz="household.tz"
          @complete="complete(t)"
          @open="open"
          @claim="act(() => tasks.claim(t.id))"
        >
          <template v-if="claimedByMe(t)" #claim>
            <button type="button" class="btn-sm" @click="act(() => tasks.release(t.id))">
              Release
            </button>
          </template>
        </TaskItem>
      </div>
    </template>

    <template v-else-if="tab === 'lists'">
      <p v-if="byList.length === 0" class="muted">No open tasks in any list.</p>
      <section v-for="g in byList" :key="g.list.id">
        <h2>{{ g.list.name }}</h2>
        <div class="rows">
          <TaskItem
            v-for="t in g.items"
            :key="t.id"
            :task="model(t)"
            :now="now"
            :tz="household.tz"
            @complete="complete(t)"
            @open="open"
            @claim="act(() => tasks.claim(t.id))"
          />
        </div>
      </section>
    </template>

    <template v-else>
      <p v-if="current.length === 0" class="muted">No suggestions waiting.</p>
      <ul class="card-list">
        <li v-for="t in current" :key="t.id">
          <strong>{{ t.title }}</strong>
          <span class="muted"> suggested by the weather</span>
          <div class="row">
            <button type="button" class="btn-primary" @click="act(() => tasks.accept(t.id))">
              Accept
            </button>
            <button type="button" class="btn-ghost" @click="act(() => tasks.skip(t.id))">
              Dismiss
            </button>
          </div>
        </li>
      </ul>
    </template>

    <UndoToast
      v-if="tasks.undo"
      :key="tasks.undo.id"
      :message="`Completed “${tasks.undo.title}”`"
      @undo="undo"
      @expire="tasks.expireUndo()"
    />
  </section>
</template>

<style scoped>
.rows {
  display: grid;
  gap: var(--space-2);
}
</style>
