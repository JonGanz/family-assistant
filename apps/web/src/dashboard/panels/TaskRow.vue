<script setup lang="ts">
import TaskItem from '../../components/tasks/TaskItem.vue'
import type { TaskItemModel } from '../../lib/tasks'
import MemberPills from '../MemberPills.vue'
import { lockCheck } from '../tasksModel'

const vLockCheck = lockCheck
defineProps<{ task: TaskItemModel; locked: boolean; now: Date; tz: string }>()
const emit = defineEmits<{ complete: [id: string] }>()
</script>

<template>
  <li v-lock-check="locked" :class="{ ro: locked }">
    <TaskItem
      :task="task"
      :now="now"
      :tz="tz"
      size="kiosk"
      @complete="(id: string) => emit('complete', id)"
    />
    <MemberPills
      :members="task.assignees.map((m) => ({ id: m.id, name: m.name, color: m.color }))"
      class="chips"
    />
  </li>
</template>

<style scoped>
.ro :deep(.check) {
  opacity: 0.35;
  cursor: not-allowed;
}
.ro :deep(.title) {
  opacity: 0.7;
}
/* Two columns: the checkbox on its own, centred against the whole content block (title, due
   date, member chips). The task item's wrapper is flattened so its children join this grid. */
li {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  column-gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  min-height: 5rem;
}
:deep(.task-item) {
  display: contents;
}
:deep(.check) {
  grid-column: 1;
  grid-row: 1 / span 2;
  align-self: center;
}
:deep(.body) {
  grid-column: 2;
  grid-row: 1;
}
:deep(.title) {
  line-height: 1.25;
}
/* Members are pill chips below the task, not square avatars. */
:deep(.people),
:deep(.claim) {
  display: none;
}
.chips {
  grid-column: 2;
  grid-row: 2;
  padding: var(--space-1) 0 0;
}
</style>
