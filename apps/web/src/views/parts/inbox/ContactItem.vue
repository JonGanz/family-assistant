<script setup lang="ts">
import { computed, reactive } from 'vue'
import type { ContactApproval, ContactSuggestionView, Member } from '../../../api/client'

const props = defineProps<{ contact: ContactSuggestionView; members: Member[]; busy?: boolean }>()
const emit = defineEmits<{ approve: [ContactApproval]; reject: [] }>()

const children = computed(() => props.members.filter((m) => m.role === 'child'))
const form = reactive({ name: props.contact.name ?? '', member: props.contact.member ?? '' })

function approve() {
  const edits: ContactApproval = {}
  if (form.name && form.name !== (props.contact.name ?? '')) edits.name = form.name
  if (form.member && form.member !== (props.contact.member ?? '')) edits.member = form.member
  emit('approve', edits)
}
</script>

<template>
  <li class="form item">
    <p>
      <strong>{{ contact.email }}</strong> at {{ contact.org_name }}
    </p>
    <label>Name <input v-model="form.name" /></label>
    <label>
      Child
      <select v-model="form.member">
        <option value="">None</option>
        <option v-for="m in children" :key="m.id" :value="m.id">{{ m.display_name }}</option>
      </select>
    </label>
    <p v-if="contact.message" class="muted">
      <RouterLink :to="{ name: 'mail-detail', params: { id: contact.message } }"
        >Open the message that suggested this</RouterLink
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
