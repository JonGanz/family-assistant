<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import type { TaskView } from '../api/client'
import TaskItem from '../components/tasks/TaskItem.vue'
import ResponsiveItems from '../app/ResponsiveItems.vue'
import { useClock } from '../composables/useClock'
import { useHouseholdStore } from '../stores/household'
import { useListsStore } from '../stores/lists'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import { addableLists, memberRefs, toTaskItem, type Viewer } from '../stores/taskMap'
import { useTasksStore, type TaskTab } from '../stores/tasks'
import {
  UiButton,
  UiChip,
  UiEmptyState,
  UiErrorState,
  UiFab,
  UiPage,
  UiSection,
  UiSegmented,
  UiSheet,
  UiSkeleton,
  UiTopAppBar,
  useSnackbar,
} from '../ui'
import { describeError } from './shared'
import TaskAddForm from './parts/TaskAddForm.vue'

type Tab = 'now' | 'lists' | 'suggested'

const session = useSessionStore()
const household = useHouseholdStore()
const members = useMembersStore()
const lists = useListsStore()
const tasks = useTasksStore()
const router = useRouter()
const now = useClock(30_000)

const tab = ref<Tab>('now')
const adding = ref(false)
const snackbar = useSnackbar()

const viewer = computed<Viewer | null>(() => {
  const m = session.member
  return m ? { id: m.id, role: m.role === 'adult' ? 'adult' : 'child' } : null
})
const TAB_LABELS: Record<Tab, string> = { now: 'Now', lists: 'Lists', suggested: 'Suggested' }
const tabOptions = computed(() =>
  (session.isAdult ? (['now', 'lists', 'suggested'] as Tab[]) : (['now', 'lists'] as Tab[])).map(
    (key) => ({ value: key, label: TAB_LABELS[key] }),
  ),
)
const refs = computed(() => memberRefs(members.items))
const canAdd = computed(() => addableLists(lists.items, viewer.value).length > 0)

// The filter chips (R2): All, Mine, then each member. Applied by the server (`member=`).
const hasMe = computed(() => !!session.member)
const activeFilter = computed(() =>
  tasks.filter === 'mine' && !hasMe.value ? 'all' : tasks.filter,
)
const showFilter = computed(() => tab.value !== 'suggested')
function pick(f: string) {
  if (f !== activeFilter.value) void tasks.setFilter(f)
}

const dataTab = computed<TaskTab>(() => (tab.value === 'lists' ? 'all' : tab.value))
function load() {
  void tasks.load(dataTab.value)
}
onMounted(async () => {
  await Promise.all([members.load(), lists.load()])
  tasks.validateFilter(members.items.map((m) => m.id))
  load()
})
watch(tab, load)

const current = computed(() => tasks.tabs[dataTab.value] ?? [])
const byList = computed(() =>
  lists.items
    .map((l) => ({ list: l, items: current.value.filter((t) => t.list === l.id) }))
    .filter((g) => g.items.length > 0),
)

const model = (t: TaskView) => toTaskItem(t, refs.value)
const open = (id: string) => router.push({ name: 'task-detail', params: { id } })

async function act(fn: () => Promise<unknown>) {
  try {
    await fn()
  } catch (e) {
    snackbar.show({ text: describeError(e, 'That did not work.') })
  }
}
const complete = (t: TaskView) => act(() => tasks.complete(t))
const claimedByMe = (t: TaskView) => !!viewer.value && t.claimed_by === viewer.value.id
const undo = () => act(() => tasks.undoLast())

// Completing is instant (D-096): the store arms a 10-second undo (§3a) and the snackbar offers it.
const UNDO_MS = 10_000
watch(
  () => tasks.undo,
  (u) => {
    if (!u) return
    snackbar.show({
      text: `Completed “${u.title}”`,
      timeoutMs: UNDO_MS,
      action: { label: 'Undo', run: undo },
    })
    setTimeout(() => {
      if (tasks.undo?.id === u.id) tasks.expireUndo()
    }, UNDO_MS)
  },
)
const loadingFirst = computed(() => tasks.loading && !tasks.tabs[dataTab.value])
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar title="Tasks" />
    </template>

    <UiSegmented
      :model-value="tab"
      :options="tabOptions"
      label="Tasks"
      @update:model-value="tab = $event as Tab"
    />

    <div v-if="showFilter" class="filter" role="group" aria-label="Show tasks for">
      <UiChip variant="filter" :selected="activeFilter === 'all'" @click="pick('all')">
        All
      </UiChip>
      <UiChip
        v-if="hasMe"
        variant="filter"
        :selected="activeFilter === 'mine'"
        @click="pick('mine')"
      >
        Mine
      </UiChip>
      <UiChip
        v-for="m in members.items"
        :key="m.id"
        variant="filter"
        :selected="activeFilter === m.id"
        @click="pick(m.id)"
      >
        <span
          class="filter__dot"
          :style="{ background: refs.get(m.id)?.color }"
          aria-hidden="true"
        />
        {{ m.display_name }}
      </UiChip>
    </div>

    <UiSkeleton v-if="loadingFirst" :lines="5" />
    <UiErrorState v-else-if="tasks.error" :message="tasks.error" @retry="load" />

    <template v-else-if="tab === 'now'">
      <UiEmptyState v-if="current.length === 0" icon="check" title="Nothing due right now" />
      <ResponsiveItems :items="current">
        <template #item="{ item }">
          <TaskItem
            :task="model(item)"
            :now="now"
            :tz="household.tz"
            @complete="complete(item)"
            @open="open"
            @claim="act(() => tasks.claim(item.id))"
          >
            <template v-if="claimedByMe(item)" #claim>
              <UiButton variant="text" @click="act(() => tasks.release(item.id))">Release</UiButton>
            </template>
          </TaskItem>
        </template>
      </ResponsiveItems>
    </template>

    <template v-else-if="tab === 'lists'">
      <UiEmptyState v-if="byList.length === 0" icon="tasks" title="No open tasks in any list" />
      <UiSection v-for="g in byList" :key="g.list.id" :heading="g.list.name">
        <ResponsiveItems :items="g.items">
          <template #item="{ item }">
            <TaskItem
              :task="model(item)"
              :now="now"
              :tz="household.tz"
              @complete="complete(item)"
              @open="open"
              @claim="act(() => tasks.claim(item.id))"
            />
          </template>
        </ResponsiveItems>
      </UiSection>
    </template>

    <template v-else>
      <UiEmptyState v-if="current.length === 0" icon="ai" title="No suggestions waiting" />
      <ResponsiveItems :items="current">
        <template #item="{ item }">
          <div class="suggestion">
            <strong>{{ item.title }}</strong>
            <span class="suggestion__by"> suggested by the weather</span>
            <div class="suggestion__actions">
              <UiButton variant="text" @click="act(() => tasks.skip(item.id))">Dismiss</UiButton>
              <UiButton variant="tonal" @click="act(() => tasks.accept(item.id))">Accept</UiButton>
            </div>
          </div>
        </template>
      </ResponsiveItems>
    </template>

    <UiButton
      v-if="tab !== 'now' && tasks.nextBefore[dataTab]"
      variant="text"
      :loading="tasks.loading"
      @click="tasks.loadMore(dataTab)"
    >
      Load more
    </UiButton>

    <div v-if="canAdd" class="fab">
      <UiFab icon="add" label="New task" extended @click="adding = true" />
    </div>
    <UiSheet v-model="adding" title="New task">
      <TaskAddForm
        :lists="lists.items"
        :members="members.items"
        :viewer="viewer"
        :tz="household.tz"
        @done="adding = false"
      />
    </UiSheet>
  </UiPage>
</template>

<style scoped>
.fab {
  position: fixed;
  right: calc(var(--ui-space-4) + env(safe-area-inset-right));
  bottom: calc(var(--ui-bottom-offset, 0px) + var(--ui-space-4) + env(safe-area-inset-bottom));
  z-index: 15;
}
.filter {
  display: flex;
  gap: var(--ui-space-2);
  overflow-x: auto;
  padding-bottom: var(--ui-space-1);
  scrollbar-width: none;
}
.filter > * {
  flex: none;
}
.filter__dot {
  display: inline-block;
  width: 0.75rem;
  height: 0.75rem;
  border-radius: var(--ui-radius-full);
}
.suggestion {
  display: grid;
  gap: var(--ui-space-2);
  padding: var(--ui-space-3) var(--ui-space-4);
}
.suggestion__by {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.suggestion__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--ui-space-2);
}
</style>
