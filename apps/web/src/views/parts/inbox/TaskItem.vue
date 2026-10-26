<script setup lang="ts">
import { computed, reactive } from 'vue'
import type { Member, TaskApproval, TaskView } from '../../../api/client'
import { fmt, fromLocalInput, toLocalInput } from '../../../components/mail/format'

const props = defineProps<{ task: TaskView; tz: string; members: Member[]; busy?: boolean }>()
const emit = defineEmits<{ approve: [TaskApproval]; reject: [] }>()

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
  <li class="settings-form item">
    <label>Title <input v-model="form.title" /></label>
    <label>Due <input v-model="form.due" type="datetime-local" /></label>
    <fieldset>
      <legend>Assignees</legend>
      <label v-for="m in members" :key="m.id" class="inline">
        <input type="checkbox" :checked="form.assignees.includes(m.id)" @change="toggle(m.id)" />
        {{ m.display_name }}
      </label>
    </fieldset>
    <p class="settings-note">
      Suggested for {{ names.length ? names.join(', ') : 'nobody yet' }}
      <span v-if="task.due_at">, due {{ fmt(task.due_at, tz) }}</span
      >.
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
