<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import type { TaskDefinitionView } from '../../api/client'
import ResponsiveItems from '../../app/ResponsiveItems.vue'
import { scheduleSummary } from '../../lib/tasks'
import { dayKey } from '../../lib/time'
import { useDefinitionsStore } from '../../stores/definitions'
import { useHouseholdStore } from '../../stores/household'
import { useListsStore } from '../../stores/lists'
import { useMembersStore } from '../../stores/members'
import { UiButton, UiChip, UiEmptyState, UiSheet, UiSkeleton } from '../../ui'
import { describeError } from '../shared'
import ConfirmDialog from './settings-mail/ConfirmDialog.vue'
import PageFab from './settings-mail/PageFab.vue'
import PageNotices from './settings-mail/PageNotices.vue'
import { useConfirm } from './settings-mail/useConfirm'
import DefinitionEditor from './DefinitionEditor.vue'
import { formFromView, toPatch } from './definitionForm'

const route = useRoute()
const defs = useDefinitionsStore()
const lists = useListsStore()
const members = useMembersStore()
const household = useHouseholdStore()
const editing = ref<string | null>(typeof route.query.def === 'string' ? route.query.def : null)
const error = ref('')
const confirm = useConfirm()
const current = () => defs.items.find((d) => d.id === editing.value) ?? null
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
const remove = (d: TaskDefinitionView) =>
  confirm.ask(`Delete “${d.title}”?`, 'Open tasks already made from it stay.', () =>
    act(() => defs.remove(d.id)),
  )
</script>

<template>
  <div class="panel">
    <PageNotices
      :load-error="defs.error"
      :conflict="defs.conflict && editing === null"
      :error="error"
      @reload="defs.load()"
    />
    <p v-if="lists.loaded && lists.items.length === 0" class="meta">
      Create a list first (Lists section).
    </p>
    <UiSkeleton v-if="!defs.loaded && !defs.error" :lines="3" />
    <UiEmptyState
      v-else-if="defs.loaded && !defs.items.length"
      icon="routines"
      title="No chores or schedules yet."
    />
    <ResponsiveItems :items="defs.items">
      <template #item="{ item: d }">
        <div class="item" :data-testid="`def-${d.title}`">
          <div class="item__head">
            <strong class="item__name">{{ d.title }}</strong>
            <UiChip v-if="d.paused" icon="info">paused</UiChip>
            <UiChip v-if="d.condition_ok === false" icon="weather">waiting for weather</UiChip>
          </div>
          <p class="meta">
            {{ listName(d.list) }} · {{ scheduleSummary(d.schedule) }} · {{ d.strategy }}
            <template v-if="d.next_due_at"> · next {{ d.next_due_at.slice(0, 10) }}</template>
          </p>
          <div class="item__row">
            <UiButton variant="text" @click="editing = d.id">Edit</UiButton>
            <UiButton variant="text" @click="togglePause(d)">
              {{ d.paused ? 'Resume' : 'Pause' }}
            </UiButton>
            <UiButton variant="text" @click="remove(d)">Delete</UiButton>
          </div>
        </div>
      </template>
    </ResponsiveItems>

    <PageFab v-if="lists.items.length" label="Add a chore or schedule" @click="editing = 'new'" />
    <UiSheet
      :model-value="editing !== null"
      :title="editing === 'new' ? 'New chore or schedule' : 'Edit chore or schedule'"
      @update:model-value="editing = null"
    >
      <DefinitionEditor
        v-if="editing !== null"
        :key="editing"
        :definition="current()"
        :today="today"
        @done="editing = null"
      />
    </UiSheet>
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
.item__head,
.item__row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.item__name {
  flex: 1;
  font: var(--ui-type-title-md);
}
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>
