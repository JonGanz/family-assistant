<script setup lang="ts">
import ErrorBanner from '../components/ErrorBanner.vue'
import { computed, onMounted, ref } from 'vue'
import type { InboxItem, InboxKind } from '../api/client'
import { useHouseholdStore } from '../stores/household'
import { itemId, useInboxStore, type Approval } from '../stores/inbox'
import { useMembersStore } from '../stores/members'
import { useSourcesStore } from '../stores/sources'
import BillItem from './parts/inbox/BillItem.vue'
import ContactItem from './parts/inbox/ContactItem.vue'
import EventItem from './parts/inbox/EventItem.vue'
import TaskItem from './parts/inbox/TaskItem.vue'
import './parts/settings.css'

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
const shown = computed(() => inbox.items.filter((i) => i.kind === tab.value))

onMounted(() => {
  void inbox.load()
  void members.load()
  void sources.load()
})

const key = (i: InboxItem) => `${i.kind}:${itemId(i)}`
const approve = (kind: InboxKind, id: string, edits: Approval) => inbox.approve(kind, id, edits)
</script>

<template>
  <section>
    <h1>Inbox</h1>
    <div role="tablist" class="tabs">
      <button
        v-for="t in TABS"
        :key="t.kind"
        type="button"
        role="tab"
        :aria-selected="tab === t.kind"
        :class="{ on: tab === t.kind }"
        @click="tab = t.kind"
      >
        {{ t.label }} ({{ inbox.counts[t.kind] }})
      </button>
    </div>
    <ErrorBanner v-if="inbox.error" :message="inbox.error" />
    <p v-if="inbox.loaded && !shown.length" class="settings-note">Nothing waiting here.</p>
    <ul class="settings-list">
      <template v-for="i in shown" :key="key(i)">
        <EventItem
          v-if="i.kind === 'event'"
          :ev="i.event"
          :tz="household.tz"
          :calendars="sources.calendars"
          :sources="sources.sources"
          :members="members.items"
          @approve="(e) => approve('event', i.event.id, e)"
          @reject="inbox.reject('event', i.event.id)"
        />
        <BillItem
          v-else-if="i.kind === 'bill'"
          :bill="i.bill"
          @approve="(e) => approve('bill', i.bill.id, e)"
          @reject="inbox.reject('bill', i.bill.id)"
        />
        <TaskItem
          v-else-if="i.kind === 'task'"
          :task="i.task"
          :tz="household.tz"
          :members="members.items"
          @approve="(e) => approve('task', i.task.id, e)"
          @reject="inbox.reject('task', i.task.id)"
        />
        <ContactItem
          v-else
          :contact="i.contact"
          :members="members.items"
          @approve="(e) => approve('contact', i.contact.id, e)"
          @reject="inbox.reject('contact', i.contact.id)"
        />
      </template>
    </ul>
  </section>
</template>

<style scoped>
.tabs {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}
.tabs button {
  min-height: 44px;
  padding: 0 var(--space-3);
  background: transparent;
  color: inherit;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}
.tabs button.on {
  border-color: var(--color-accent);
  font-weight: 600;
}
</style>
