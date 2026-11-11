<script setup lang="ts">
import { computed, reactive } from 'vue'
import { UiButton, UiCard, UiSelect, UiTextField } from '../../../ui'
import type { ContactApproval, ContactSuggestionView, Member } from '../../../api/client'

const props = defineProps<{ contact: ContactSuggestionView; members: Member[]; busy?: boolean }>()
const emit = defineEmits<{ approve: [ContactApproval]; reject: []; discard: [] }>()

const children = computed(() => props.members.filter((m) => m.role === 'child'))
const childOptions = computed(() => [
  { value: '', label: 'None' },
  ...children.value.map((m) => ({ value: m.id, label: m.display_name })),
])
const form = reactive({ name: props.contact.name ?? '', member: props.contact.member ?? '' })

function approve() {
  const edits: ContactApproval = {}
  if (form.name && form.name !== (props.contact.name ?? '')) edits.name = form.name
  if (form.member && form.member !== (props.contact.member ?? '')) edits.member = form.member
  emit('approve', edits)
}
</script>

<template>
  <li class="item">
    <UiCard variant="filled">
      <template #header>
        <div class="head">
          <span>{{ contact.email }} at {{ contact.org_name }}</span>
          <slot name="header-trailing" />
        </div>
      </template>
      <div class="fields">
        <UiTextField v-model="form.name" label="Name" />
        <UiSelect v-model="form.member" label="Child" :options="childOptions" />
        <p v-if="contact.message" class="note">
          <RouterLink :to="{ name: 'mail-detail', params: { id: contact.message } }"
            >Open the message that suggested this</RouterLink
          >
        </p>
      </div>
      <template #actions>
        <UiButton
          variant="text"
          :disabled="busy"
          title="Not needed; doesn't count against the AI"
          @click="emit('discard')"
          >Discard</UiButton
        >
        <UiButton
          variant="text"
          :disabled="busy"
          title="The suggestion was wrong (used to measure accuracy)"
          @click="emit('reject')"
          >Reject</UiButton
        >
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
