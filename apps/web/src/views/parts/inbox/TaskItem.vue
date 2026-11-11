<script setup lang="ts">
import { computed, reactive } from 'vue'
import type { Member, TaskApproval, TaskView } from '../../../api/client'
import { UiButton, UiCard, UiCheckbox, UiTextField } from '../../../ui'
import { fmt, fromLocalInput, toLocalInput } from '../../../components/mail/format'

const props = defineProps<{
  task: TaskView
  message?: string | null
  tz: string
  members: Member[]
  busy?: boolean
}>()
const emit = defineEmits<{ approve: [TaskApproval]; reject: []; discard: [] }>()

const form = reactive({
  title: props.task.title,
  due: props.task.due_at ? toLocalInput(props.task.due_at, props.tz) : '',
  assignees: [...props.task.assignees],
})
const names = computed(() =>
  props.members.filter((m) => props.task.assignees.includes(m.id)).map((m) => m.display_name),
)

function toggle(id: string) {
  form.assignees = form.assignees.includes(id)
    ? form.assignees.filter((x) => x !== id)
    : [...form.assignees, id]
}
function approve() {
  const edits: TaskApproval = {}
  if (form.title !== props.task.title) edits.title = form.title
  const orig = props.task.due_at ? toLocalInput(props.task.due_at, props.tz) : ''
  if (form.due && form.due !== orig) edits.due_at = fromLocalInput(form.due, props.tz)
  const same =
    form.assignees.length === props.task.assignees.length &&
    form.assignees.every((a) => props.task.assignees.includes(a))
  if (!same) edits.assignees = form.assignees
  emit('approve', edits)
}
</script>

<template>
  <li class="item">
    <UiCard variant="filled">
      <template #header>
        <div class="head">
          <span>New task</span>
          <slot name="header-trailing" />
        </div>
      </template>
      <div class="fields">
        <UiTextField v-model="form.title" label="Title" />
        <UiTextField v-model="form.due" label="Due" type="datetime-local" />
        <fieldset class="checks">
          <legend>Assignees</legend>
          <UiCheckbox
            v-for="m in members"
            :key="m.id"
            :model-value="form.assignees.includes(m.id)"
            :label="m.display_name"
            @update:model-value="toggle(m.id)"
          />
        </fieldset>
        <p class="note">
          Suggested for {{ names.length ? names.join(', ') : 'nobody yet' }}
          <span v-if="task.due_at">, due {{ fmt(task.due_at, tz) }}</span
          >.
          <RouterLink v-if="message" :to="{ name: 'mail-detail', params: { id: message } }"
            >Open message</RouterLink
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
