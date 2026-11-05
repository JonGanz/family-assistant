<script setup lang="ts">
import { ref } from 'vue'
import type { MailActionView } from '../../../api/client'
import { fmt } from '../../../components/mail/format'
import {
  UiButton,
  UiDialog,
  UiList,
  UiListItem,
  UiMenu,
  UiTextField,
  type MenuItem,
} from '../../../ui'

const props = defineProps<{ actions: MailActionView[]; canAct: boolean; tz: string }>()
const emit = defineEmits<{
  act: [action: 'archive' | 'mark_read' | 'label', label?: string]
  undo: [id: string]
}>()

const items: MenuItem[] = [
  { id: 'archive', label: 'Archive' },
  { id: 'mark_read', label: 'Mark read' },
  { id: 'label', label: 'Add label…' },
]
const labelling = ref(false)
const label = ref('')
const now = Date.now()
const canUndo = (a: MailActionView) => !a.undone_at && !a.error && Date.parse(a.undo_until) > now

function select(id: string) {
  if (id === 'label') labelling.value = true
  else emit('act', id as 'archive' | 'mark_read')
}
function addLabel() {
  const name = label.value.trim()
  if (!name) return
  emit('act', 'label', name)
  labelling.value = false
  label.value = ''
}
function describe(a: MailActionView): string {
  const name = a.action.replace('_', ' ')
  return `${name}${a.label ? ` "${a.label}"` : ''}${a.automatic ? ' (automatic)' : ''}`
}
function status(a: MailActionView): string {
  const when = `at ${fmt(a.done_at, props.tz)}`
  if (a.error) return `${when}. ${a.error}`
  return a.undone_at ? `${when}, undone` : when
}
</script>

<template>
  <div class="actions">
    <UiMenu v-if="canAct" :items="items" label="Mail actions" @select="select">
      <template #activator="{ toggle, attrs }">
        <UiButton variant="tonal" icon="mail" v-bind="attrs" @click="toggle">Mail actions</UiButton>
      </template>
    </UiMenu>
    <p v-else class="note">This source is read-only, so mail actions are unavailable.</p>

    <UiList v-if="actions.length" aria-label="Undo list">
      <UiListItem v-for="a in actions" :key="a.id" :headline="describe(a)" :supporting="status(a)">
        <template v-if="canUndo(a)" #trailing>
          <UiButton variant="text" icon="undo" @click="emit('undo', a.id)">Undo</UiButton>
        </template>
      </UiListItem>
    </UiList>

    <UiDialog v-model="labelling" title="Add label">
      <UiTextField v-model="label" label="Label name" />
      <template #actions>
        <UiButton variant="text" @click="labelling = false">Cancel</UiButton>
        <UiButton :disabled="!label.trim()" @click="addLabel">Add label</UiButton>
      </template>
    </UiDialog>
  </div>
</template>

<style scoped>
.actions {
  display: grid;
  gap: var(--ui-space-3);
}
.note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
</style>
