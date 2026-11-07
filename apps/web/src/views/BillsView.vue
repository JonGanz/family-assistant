<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import type { BillView } from '../api/client'
import ResponsiveItems from '../app/ResponsiveItems.vue'
import { fmtDay } from '../components/mail/format'
import { formatMoney, useBillsStore, type BillFilter } from '../stores/bills'
import {
  UiButton,
  UiChip,
  UiEmptyState,
  UiErrorState,
  UiFilterChips,
  UiPage,
  UiSection,
  UiSkeleton,
  UiTopAppBar,
  useSnackbar,
} from '../ui'
import { describeError } from './shared'

const bills = useBillsStore()
const router = useRouter()
const snackbar = useSnackbar()

const FILTERS: { value: BillFilter; label: string }[] = [
  { value: 'proposed', label: 'Proposed' },
  { value: 'open', label: 'Open' },
  { value: 'paid', label: 'Paid' },
  { value: 'dismissed', label: 'Dismissed' },
]
const GROUPS: { status: string; heading: string }[] = [
  { status: 'open', heading: 'Open' },
  { status: 'proposed', heading: 'Proposed' },
  { status: 'paid', heading: 'Paid' },
  { status: 'dismissed', heading: 'Dismissed' },
]
const groups = computed(() =>
  GROUPS.filter((g) => !bills.filter.length || (bills.filter as string[]).includes(g.status))
    .map((g) => ({ ...g, items: bills.items.filter((b) => b.status === g.status) }))
    .filter((g) => g.items.length > 0),
)

onMounted(() => void bills.load())
watch(
  () => bills.filter,
  () => void bills.load(),
)

function openExternal(url?: string | null) {
  if (url) window.open(url, '_blank', 'noopener,noreferrer')
}

/** Dismiss and mark-open are instant with an Undo that restores the previous state. */
async function change(b: BillView, status: 'open' | 'dismissed') {
  const before = b.status
  const id = b.id
  try {
    await bills.patch(b, { status })
  } catch (e) {
    snackbar.show({ text: describeError(e) })
    return
  }
  snackbar.show({
    text: status === 'dismissed' ? 'Bill dismissed' : 'Bill marked open',
    action: {
      label: 'Undo',
      run: async () => {
        const fresh = bills.items.find((x) => x.id === id)
        if (!fresh || (before !== 'open' && before !== 'proposed' && before !== 'dismissed')) return
        try {
          await bills.patch(fresh, { status: before as 'open' | 'dismissed' })
        } catch (e) {
          snackbar.show({ text: describeError(e) })
        }
      },
    },
  })
}
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar title="Bills" />
    </template>

    <UiFilterChips
      :model-value="bills.filter"
      :options="FILTERS"
      label="Show"
      @update:model-value="bills.filter = $event as BillFilter[]"
    />

    <UiSkeleton v-if="bills.loading && !bills.items.length" :lines="5" />
    <UiErrorState v-else-if="bills.error" :message="bills.error" @retry="bills.load()" />
    <UiEmptyState
      v-else-if="!bills.loading && !bills.items.length"
      icon="bills"
      title="No bills."
    />

    <UiSection v-for="g in groups" :key="g.status" :heading="g.heading">
      <ResponsiveItems :items="g.items">
        <template #item="{ item: b }">
          <div class="bill">
            <div class="bill__top">
              <strong>{{ b.payee }}</strong>
              <span class="bill__amount">{{ formatMoney(b.amount_minor, b.currency) }}</span>
            </div>
            <p v-if="b.due_on" class="bill__due">due {{ fmtDay(b.due_on) }}</p>
            <div class="bill__chips">
              <UiChip
                v-if="b.task"
                icon="tasks"
                @click="router.push({ name: 'task-detail', params: { id: b.task } })"
                >Pay task: {{ b.task_done ? 'done' : 'open' }}</UiChip
              >
              <UiChip
                v-if="b.message"
                icon="mail"
                @click="router.push({ name: 'mail-detail', params: { id: b.message } })"
                >Message</UiChip
              >
              <UiChip
                v-if="b.external_url"
                icon="open_external"
                @click="openExternal(b.external_url)"
                >Original</UiChip
              >
            </div>
            <div v-if="b.status === 'proposed' || b.status === 'open'" class="bill__actions">
              <UiButton variant="text" @click="change(b, 'dismissed')">Dismiss</UiButton>
              <UiButton v-if="b.status === 'proposed'" variant="tonal" @click="change(b, 'open')"
                >Mark open</UiButton
              >
            </div>
          </div>
        </template>
      </ResponsiveItems>
    </UiSection>

    <UiButton
      v-if="bills.nextBefore"
      variant="text"
      :loading="bills.loading"
      @click="bills.loadMore()"
    >
      Load more
    </UiButton>
  </UiPage>
</template>

<style scoped>
.bill {
  display: grid;
  gap: var(--ui-space-1);
  padding: var(--ui-space-3) var(--ui-space-4);
}
.bill__top {
  display: flex;
  justify-content: space-between;
  gap: var(--ui-space-3);
  font: var(--ui-type-title-sm);
}
.bill__amount {
  font-variant-numeric: tabular-nums;
}
.bill__due {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.bill__chips {
  display: flex;
  flex-wrap: wrap;
  column-gap: var(--ui-space-2);
}
.bill__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--ui-space-2);
}
</style>
