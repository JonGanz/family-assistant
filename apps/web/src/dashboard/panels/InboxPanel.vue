<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { inboxApi, type InboxItem, type InboxKind } from '../../api/mailSettings'
import { ApiError } from '../../api/http'
import { useDashboardStore } from '../../stores/dashboard'
import { S } from '../strings'
import type { PanelProps } from '../types'

const props = defineProps<PanelProps>()
const store = useDashboardStore()

const counts = computed(
  () => props.data.inbox as (Record<InboxKind, number> & { total: number }) | undefined,
)
const total = computed(() => Number(counts.value?.total ?? 0))
const elevated = computed(() => (store.elevatedUntil ?? 0) > props.now.getTime())
const items = ref<InboxItem[]>([])
const message = ref('')

async function load(): Promise<void> {
  if (!elevated.value) {
    items.value = []
    return
  }
  try {
    items.value = (await inboxApi.list()).items
  } catch {
    items.value = []
  }
}

// The snapshot object is replaced on every refetch (including `inbox.updated`).
watch(() => [props.data, elevated.value], load, { immediate: true })

const reviewable = computed(() => items.value.filter((i) => i.kind !== 'contact'))

function ident(i: InboxItem): { kind: InboxKind; id: string; title: string; detail: string } {
  switch (i.kind) {
    case 'event':
      return {
        kind: 'event',
        id: i.event.id,
        title: i.event.pending_cancel ? `Cancelled: ${i.event.title}` : i.event.title,
        detail: i.event.write_error ?? i.event.from_label ?? '',
      }
    case 'bill':
      return {
        kind: 'bill',
        id: i.bill.id,
        title: i.bill.payee,
        detail: `${(Number(i.bill.amount_minor) / 100).toFixed(2)} ${i.bill.currency}${i.bill.due_on ? ` · due ${i.bill.due_on}` : ''}`,
      }
    case 'task':
      return { kind: 'task', id: i.task.id, title: i.task.title, detail: '' }
    case 'contact':
      return {
        kind: 'contact',
        id: i.contact.id,
        title: i.contact.name ?? i.contact.email,
        detail: i.contact.org_name,
      }
  }
}

async function decide(i: InboxItem, approve: boolean): Promise<void> {
  const { kind, id } = ident(i)
  message.value = ''
  try {
    if (approve) await inboxApi.approve(kind, id)
    else await inboxApi.reject(kind, id)
  } catch (e) {
    message.value = e instanceof ApiError && e.status === 409 ? S.inboxStale : S.inboxFailed
  }
  await store.refresh()
  await load()
}

const failed = (i: InboxItem) => i.kind === 'event' && !!i.event.write_error
</script>

<template>
  <section class="inbox" aria-label="Inbox">
    <header>
      <h2>{{ S.panelNames['inbox'] }}</h2>
      <span v-if="total > 0" class="total" data-testid="inbox-total">{{ total }}</span>
    </header>
    <p v-if="total === 0" class="none">{{ S.inboxEmpty }}</p>
    <template v-else>
      <ul class="counts" data-testid="inbox-counts">
        <template v-for="k in ['event', 'bill', 'task', 'contact']" :key="k">
          <li v-if="(counts?.[k as InboxKind] ?? 0) > 0">
            {{ S.inboxCounts[k] }}: {{ counts?.[k as InboxKind] }}
          </li>
        </template>
      </ul>
      <p v-if="!elevated" class="none" data-testid="inbox-locked">{{ S.inboxUnlock }}</p>
    </template>
    <ul v-if="elevated" class="items">
      <li v-for="i in reviewable" :key="ident(i).kind + ident(i).id">
        <div class="what">
          <strong>{{ ident(i).title }}</strong>
          <span :class="{ err: failed(i) }">{{ ident(i).detail }}</span>
        </div>
        <button type="button" @click="decide(i, true)">
          {{ failed(i) ? S.retryWrite : S.approve }}
        </button>
        <button type="button" @click="decide(i, false)">{{ S.reject }}</button>
      </li>
    </ul>
    <p v-if="message" role="alert">{{ message }}</p>
  </section>
</template>

<style scoped>
.inbox {
  display: grid;
  gap: var(--space-3);
  align-content: start;
}
header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
h2 {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: 600;
}
.total {
  padding: 0 var(--space-3);
  border-radius: 999px;
  background: var(--color-accent);
  color: var(--color-bg, #fff);
}
.none {
  margin: 0;
  color: var(--color-muted);
  font-size: var(--font-size-base);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}
.counts {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}
.items li {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}
.what {
  flex: 1;
  display: grid;
}
.err {
  color: var(--color-danger);
}
button {
  min-width: var(--min-touch, 48px);
  min-height: var(--min-touch, 48px);
  border-radius: var(--radius-md);
  cursor: pointer;
}
</style>
