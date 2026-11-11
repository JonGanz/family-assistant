<script setup lang="ts">
import MemberChip from '../../components/MemberChip.vue'
import TaskItem from '../../components/tasks/TaskItem.vue'
import type { TaskItemModel } from '../../lib/tasks'
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
    <span v-if="task.assignees.length" class="chips">
      <MemberChip
        v-for="m in task.assignees"
        :key="m.id"
        :name="m.name"
        :color="m.color"
        variant="app"
      />
    </span>
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
/* Checkbox centred on the first line of the title, also when the title wraps. */
:deep(.task-item) {
  align-items: flex-start;
}
:deep(.check) {
  margin-top: calc((1.25 * var(--kiosk-font-base) - 3rem) / 2);
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
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  padding: 0 var(--space-4) var(--space-2) calc(var(--space-4) + 3rem + var(--space-4));
}
.chips :deep(.member-chip) {
  padding: 0 var(--space-3);
  font-size: var(--kiosk-font-small);
  line-height: 1.6;
}
</style>
