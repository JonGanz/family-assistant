<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Member, TaskListView } from '../../api/client'
import { useTasksStore } from '../../stores/tasks'
import { addableLists, assignableMembers, type Viewer } from '../../stores/taskMap'
import { UiButton, UiSelect, UiTextField } from '../../ui'
import { describeError } from '../shared'
import { toInstant } from './dateInput'

const props = defineProps<{
  lists: TaskListView[]
  members: Member[]
  viewer: Viewer | null
  tz: string
}>()
const emit = defineEmits<{ done: [] }>()

const tasks = useTasksStore()
const allowed = computed(() => addableLists(props.lists, props.viewer))
const people = computed(() => assignableMembers(props.members, props.viewer))
const isChild = computed(() => props.viewer?.role === 'child')

const title = ref('')
const list = ref(allowed.value[0]?.id ?? '')
const date = ref('')
const time = ref('')
const assignee = ref(isChild.value ? (props.viewer?.id ?? '') : '')
const error = ref('')
const adding = ref(false)
const listOptions = computed(() => allowed.value.map((l) => ({ value: l.id, label: l.name })))
const assigneeOptions = computed(() => [
  { value: '', label: 'Anyone (household)' },
  ...people.value.map((m) => ({ value: m.id, label: m.display_name })),
])

async function submit() {
  if (adding.value) return
  error.value = ''
  adding.value = true
  try {
    const assignees =
      isChild.value && props.viewer ? [props.viewer.id] : assignee.value ? [assignee.value] : []
    await tasks.create({
      list: list.value,
      title: title.value.trim(),
      notes: null,
      due_at: toInstant(date.value, time.value, props.tz),
      assignees,
      steps: [],
      priority_flag: null,
      calendar_display: null,
      member: null,
    })
    title.value = ''
    emit('done')
  } catch (e) {
    error.value = describeError(e, 'Could not add the task.')
  } finally {
    adding.value = false
  }
}
</script>

<template>
  <form class="add-form" @submit.prevent="submit">
    <UiTextField v-model="title" label="Task" name="title" required maxlength="200" />
    <UiSelect v-model="list" label="List" :options="listOptions" />
    <UiTextField v-model="date" label="Due date" name="due_date" type="date" />
    <UiTextField v-if="date" v-model="time" label="Due time" name="due_time" type="time" />
    <UiSelect v-if="!isChild" v-model="assignee" label="For" :options="assigneeOptions" />
    <p v-if="error" class="add-form__error" role="alert">{{ error }}</p>
    <div class="add-form__actions">
      <UiButton variant="text" @click="emit('done')">Cancel</UiButton>
      <UiButton type="submit" :loading="adding" :disabled="!list || !title.trim()">Add</UiButton>
    </div>
  </form>
</template>

<style scoped>
.add-form {
  display: grid;
  gap: var(--ui-space-4);
}
.add-form__error {
  margin: 0;
  color: var(--ui-error);
  font: var(--ui-type-body-md);
}
.add-form__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--ui-space-2);
}
</style>
