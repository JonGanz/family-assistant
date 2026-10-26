<script setup lang="ts">
import { onMounted, watch } from 'vue'
import { fmtDay } from '../components/mail/format'
import { formatMoney, useBillsStore, type BillFilter } from '../stores/bills'
import './parts/settings.css'

const bills = useBillsStore()
const FILTERS: { v: BillFilter; label: string }[] = [
  { v: '', label: 'All' },
  { v: 'proposed', label: 'Proposed' },
  { v: 'open', label: 'Open' },
  { v: 'paid', label: 'Paid' },
  { v: 'dismissed', label: 'Dismissed' },
]
onMounted(() => void bills.load())
watch(
  () => bills.filter,
  () => void bills.load(),
)
</script>

<template>
  <section>
    <h1>Bills</h1>
    <label>
      Show
      <select v-model="bills.filter">
        <option v-for="f in FILTERS" :key="f.v" :value="f.v">{{ f.label }}</option>
      </select>
    </label>
    <p v-if="bills.error" class="settings-error" role="alert">{{ bills.error }}</p>
    <p v-if="!bills.loading && !bills.items.length" class="settings-note">No bills.</p>
    <ul class="settings-list">
      <li v-for="b in bills.items" :key="b.id">
        <strong>{{ b.payee }}</strong
        >: {{ formatMoney(b.amount_minor, b.currency) }}
        <span v-if="b.due_on">, due {{ fmtDay(b.due_on) }}</span>
        <span class="badge">{{ b.status }}</span>
        <div class="settings-note">
          <template v-if="b.task">
            Pay task:
            <RouterLink :to="{ name: 'task-detail', params: { id: b.task } }">{{
              b.task_done ? 'done' : 'open'
            }}</RouterLink>
          </template>
          <template v-if="b.message">
            <RouterLink :to="{ name: 'mail-detail', params: { id: b.message } }">
              Message</RouterLink
            >
          </template>
          <a v-if="b.external_url" :href="b.external_url" target="_blank" rel="noopener noreferrer">
            Original ↗</a
          >
        </div>
        <div v-if="b.status === 'proposed' || b.status === 'open'" class="row">
          <button type="button" @click="bills.patch(b, { status: 'dismissed' })">Dismiss</button>
          <button
            v-if="b.status === 'proposed'"
            type="button"
            @click="bills.patch(b, { status: 'open' })"
          >
            Mark open
          </button>
        </div>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.row {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-2);
}
</style>
