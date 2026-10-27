<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Member, TaskListView } from '../../api/client'
import { useTasksStore } from '../../stores/tasks'
import { addableLists, assignableMembers, type Viewer } from '../../stores/taskMap'
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

async function submit() {
  error.value = ''
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
  }
}
</script>

<template>
  <form class="form add-form" @submit.prevent="submit">
    <label>Task <input v-model="title" name="title" required maxlength="200" /></label>
    <label>
      List
      <select v-model="list" name="list" required>
        <option v-for="l in allowed" :key="l.id" :value="l.id">{{ l.name }}</option>
      </select>
    </label>
    <label>Due date <input v-model="date" name="due_date" type="date" /></label>
    <label v-if="date">Due time <input v-model="time" name="due_time" type="time" /></label>
    <label v-if="!isChild">
      For
      <select v-model="assignee" name="assignee">
        <option value="">Anyone (household)</option>
        <option v-for="m in people" :key="m.id" :value="m.id">{{ m.display_name }}</option>
      </select>
    </label>
    <p v-if="error" class="text-danger" role="alert">{{ error }}</p>
    <div class="row">
      <button type="submit" :disabled="!list || !title.trim()">Add</button>
      <button type="button" @click="emit('done')">Cancel</button>
    </div>
  </form>
</template>

<style scoped>
.row {
  display: flex;
  gap: var(--space-2);
}
</style>
