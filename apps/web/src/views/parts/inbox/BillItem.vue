<script setup lang="ts">
import { reactive } from 'vue'
import type { BillPatch, BillView } from '../../../api/client'
import { fmtDay } from '../../../components/mail/format'
import { formatMoney } from '../../../stores/bills'
import { UiButton, UiCard, UiTextField } from '../../../ui'

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
  <li class="item">
    <UiCard variant="filled">
      <template #header>
        <div class="head">
          <span
            >{{ formatMoney(bill.amount_minor, bill.currency)
            }}<template v-if="bill.due_on"> due {{ fmtDay(bill.due_on) }}</template></span
          >
          <slot name="header-trailing" />
        </div>
      </template>
      <div class="fields">
        <UiTextField v-model="form.payee" label="Payee" />
        <UiTextField v-model="form.due" label="Due" type="date" />
        <p class="note">
          Confidence {{ Math.round(bill.confidence * 100) }}%.
          <RouterLink
            v-if="bill.message"
            :to="{ name: 'mail-detail', params: { id: bill.message } }"
            >Open message</RouterLink
          >
        </p>
      </div>
      <template #actions>
        <UiButton variant="text" :disabled="busy" @click="emit('reject')">Reject</UiButton>
        <UiButton :disabled="busy" @click="approve">Approve</UiButton>
      </template>
    </UiCard>
  </li>
</template>

<style scoped>
.item {
  list-style: none;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ui-space-2);
  font: var(--ui-type-title-sm);
}
.fields {
  display: grid;
  gap: var(--ui-space-3);
}
.note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.err {
  margin: 0;
  color: var(--ui-error);
  font: var(--ui-type-body-md);
}
.checks {
  display: grid;
  margin: 0;
  padding: 0;
  border: 0;
}
.checks legend {
  padding: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-label-md);
}
</style>
