<script setup lang="ts">
import { reactive } from 'vue'
import type { BillPatch, BillView } from '../../../api/client'
import { fmtDay } from '../../../components/mail/format'
import { formatMoney } from '../../../stores/bills'

const props = defineProps<{ bill: BillView; busy?: boolean }>()
const emit = defineEmits<{ approve: [BillPatch]; reject: [] }>()

const form = reactive({ payee: props.bill.payee, due: props.bill.due_on ?? '' })

function approve() {
  const edits: BillPatch = {}
  if (form.payee !== props.bill.payee) edits.payee = form.payee
  if (form.due !== (props.bill.due_on ?? '')) edits.due_on = form.due || null
  emit('approve', edits)
}
</script>

<template>
  <li class="settings-form item">
    <p>
      <strong>{{ formatMoney(bill.amount_minor, bill.currency) }}</strong>
      <span v-if="bill.due_on"> due {{ fmtDay(bill.due_on) }}</span>
    </p>
    <label>Payee <input v-model="form.payee" /></label>
    <label>Due <input v-model="form.due" type="date" /></label>
    <p class="settings-note">
      Confidence {{ Math.round(bill.confidence * 100) }}%.
      <RouterLink v-if="bill.message" :to="{ name: 'mail-detail', params: { id: bill.message } }"
        >Open message</RouterLink
      >
    </p>
    <div class="actions">
      <button type="button" :disabled="busy" @click="approve">Approve</button>
      <button type="button" :disabled="busy" @click="emit('reject')">Reject</button>
    </div>
  </li>
</template>

<style scoped>
.item {
  max-width: none;
}
.actions {
  display: flex;
  gap: var(--space-2);
}
.actions button {
  min-height: 44px;
}
</style>
