<script setup lang="ts">
import { ref } from 'vue'
import type { MailActionView } from '../../../api/client'
import { fmt } from '../../../components/mail/format'

defineProps<{ actions: MailActionView[]; canAct: boolean; tz: string }>()
const emit = defineEmits<{
  act: [action: 'archive' | 'mark_read' | 'label', label?: string]
  undo: [id: string]
}>()
const label = ref('')
const now = Date.now()
const canUndo = (a: MailActionView) => !a.undone_at && !a.error && Date.parse(a.undo_until) > now
</script>

<template>
  <div>
    <div v-if="canAct" class="row">
      <button type="button" @click="emit('act', 'archive')">Archive</button>
      <button type="button" @click="emit('act', 'mark_read')">Mark read</button>
      <input v-model="label" placeholder="Label" aria-label="Label name" />
      <button type="button" :disabled="!label.trim()" @click="emit('act', 'label', label.trim())">
        Add label
      </button>
    </div>
    <p v-else class="settings-note">This source is read-only, so mail actions are unavailable.</p>
    <ul v-if="actions.length" class="settings-list">
      <li v-for="a in actions" :key="a.id">
        {{ a.action }}<span v-if="a.label"> "{{ a.label }}"</span>
        {{ a.automatic ? '(automatic)' : '' }} at {{ fmt(a.done_at, tz) }}
        <span v-if="a.undone_at" class="settings-note">undone</span>
        <span v-if="a.error" class="settings-error">{{ a.error }}</span>
        <button v-if="canUndo(a)" type="button" @click="emit('undo', a.id)">Undo</button>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}
</style>
