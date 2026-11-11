<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import type { InboxItem, InboxKind } from '../api/client'
import { useHouseholdStore } from '../stores/household'
import { itemId, useInboxStore, type Approval } from '../stores/inbox'
import { useMembersStore } from '../stores/members'
import { useSourcesStore } from '../stores/sources'
import {
  UiButton,
  UiChip,
  UiDialog,
  UiEmptyState,
  UiErrorState,
  UiPage,
  UiSkeleton,
  UiTabs,
  UiTopAppBar,
} from '../ui'
import AiChip from '../components/mail/AiChip.vue'
import BillItem from './parts/inbox/BillItem.vue'
import ContactItem from './parts/inbox/ContactItem.vue'
import EventItem from './parts/inbox/EventItem.vue'
import TaskItem from './parts/inbox/TaskItem.vue'

const inbox = useInboxStore()
const household = useHouseholdStore()
const members = useMembersStore()
const sources = useSourcesStore()

const TABS: { kind: InboxKind; label: string }[] = [
  { kind: 'event', label: 'Events' },
  { kind: 'bill', label: 'Bills' },
  { kind: 'task', label: 'Tasks' },
  { kind: 'contact', label: 'Contacts' },
]
const tab = ref<InboxKind>('event')
const tabOptions = computed(() =>
  TABS.map((t) => ({ value: t.kind, label: `${t.label} (${inbox.counts[t.kind]})` })),
)
const aiOnly = ref(false)
const isAi = (i: InboxItem) =>
  i.kind === 'event'
    ? i.event.extracted_by === 'llm'
    : i.kind === 'bill'
      ? i.bill.extracted_by === 'llm'
      : i.kind === 'task'
        ? i.task.extracted_by === 'llm'
        : false
const shown = computed(() =>
  inbox.items.filter((i) => i.kind === tab.value && (!aiOnly.value || isAi(i))),
)

onMounted(() => {
  void inbox.load()
  void members.load()
  void sources.load()
})

const key = (i: InboxItem) => `${i.kind}:${itemId(i)}`
// Approving is instant (the server writes the result; there is no undo for it). Rejecting
// drops the proposal for good, so it confirms first.
const approve = (kind: InboxKind, id: string, edits: Approval) => inbox.approve(kind, id, edits)
const rejecting = ref<{ kind: InboxKind; id: string } | null>(null)
const confirmOpen = computed({
  get: () => rejecting.value !== null,
  set: (v: boolean) => {
    if (!v) rejecting.value = null
  },
})
function confirmReject() {
  const r = rejecting.value
  rejecting.value = null
  if (r) void inbox.reject(r.kind, r.id)
}
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar title="Inbox" />
    </template>

    <UiTabs
      :model-value="tab"
      :options="tabOptions"
      label="Inbox"
      @update:model-value="tab = $event as InboxKind"
    />

    <div class="filters" role="group" aria-label="Filter">
      <UiChip variant="filter" :selected="aiOnly" @click="aiOnly = !aiOnly">AI only</UiChip>
    </div>

    <UiSkeleton v-if="!inbox.loaded && !inbox.error" :lines="5" />
    <UiErrorState v-else-if="inbox.error" :message="inbox.error" @retry="inbox.load()" />
    <UiEmptyState
      v-else-if="inbox.loaded && !shown.length"
      icon="inbox"
      title="Nothing waiting here."
    />

    <ul class="stack">
      <template v-for="i in shown" :key="key(i)">
        <EventItem
          v-if="i.kind === 'event'"
          :ev="i.event"
          :tz="household.tz"
          :calendars="sources.calendars"
          :sources="sources.sources"
          :members="members.items"
          @approve="(e) => approve('event', i.event.id, e)"
          @reject="rejecting = { kind: 'event', id: i.event.id }"
        >
          <template #header-trailing>
            <AiChip v-if="i.event.extracted_by === 'llm'" :confidence="i.event.confidence" />
          </template>
        </EventItem>
        <BillItem
          v-else-if="i.kind === 'bill'"
          :bill="i.bill"
          @approve="(e) => approve('bill', i.bill.id, e)"
          @reject="rejecting = { kind: 'bill', id: i.bill.id }"
        >
          <template #header-trailing>
            <AiChip v-if="i.bill.extracted_by === 'llm'" :confidence="i.bill.confidence" />
          </template>
        </BillItem>
        <TaskItem
          v-else-if="i.kind === 'task'"
          :task="i.task"
          :message="i.message"
          :tz="household.tz"
          :members="members.items"
          @approve="(e) => approve('task', i.task.id, e)"
          @reject="rejecting = { kind: 'task', id: i.task.id }"
        >
          <template #header-trailing>
            <AiChip v-if="i.task.extracted_by === 'llm'" />
          </template>
        </TaskItem>
        <ContactItem
          v-else
          :contact="i.contact"
          :members="members.items"
          @approve="(e) => approve('contact', i.contact.id, e)"
          @reject="rejecting = { kind: 'contact', id: i.contact.id }"
        />
      </template>
    </ul>
    <UiButton v-if="inbox.nextBefore" variant="text" @click="inbox.loadMore()">Load more</UiButton>

    <UiDialog v-model="confirmOpen" title="Reject this proposal?">
      <p>It is dropped and will not be suggested again.</p>
      <template #actions>
        <UiButton variant="text" @click="rejecting = null">Cancel</UiButton>
        <UiButton @click="confirmReject">Reject</UiButton>
      </template>
    </UiDialog>
  </UiPage>
</template>

<style scoped>
.filters {
  display: flex;
  flex-wrap: wrap;
  column-gap: var(--ui-space-2);
}
.stack {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--ui-space-3);
  margin: 0;
  padding: 0;
}
@media (min-width: 840px) {
  .stack {
    grid-template-columns: repeat(auto-fill, minmax(22rem, 1fr));
    align-items: start;
  }
}
</style>
