<script setup lang="ts">
import Tabs from '../components/Tabs.vue'
import PageHeader from '../components/PageHeader.vue'
import StateBlock from '../components/StateBlock.vue'
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
const tabItems = computed(() =>
  TABS.map((t) => ({ key: t.kind, label: `${t.label} (${inbox.counts[t.kind]})` })),
)
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
  <section aria-labelledby="page-title">
    <PageHeader title="Inbox" />
    <Tabs v-model="tab" :tabs="tabItems" label="Inbox" />
    <StateBlock
      :loading="!inbox.loaded && !inbox.error"
      :error="inbox.error"
      :empty="inbox.loaded && !shown.length"
      empty-text="Nothing waiting here."
    />
    <ul class="card-list">
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
