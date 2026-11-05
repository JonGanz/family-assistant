<script setup lang="ts">
import { computed } from 'vue'
import Avatar from '../Avatar.vue'
import { UiButton, UiIcon } from '../../ui'
import { dueLabel, isOverdue, type TaskItemModel } from '../../lib/tasks'

const props = defineProps<{
  task: TaskItemModel
  now: Date
  tz: string
  size?: 'normal' | 'kiosk'
}>()
const emit = defineEmits<{ complete: [id: string]; open: [id: string]; claim: [id: string] }>()

const done = computed(() => props.task.status === 'done')
const due = computed(() => dueLabel(props.task.dueAt, props.now, props.tz))
const overdue = computed(() => !done.value && isOverdue(props.task.dueAt, props.now))
</script>

<template>
  <div class="task-item" :class="[size ?? 'normal', { done, overdue }]">
    <input
      class="check"
      type="checkbox"
      :checked="done"
      :aria-label="`Complete ${task.title}`"
      @change="emit('complete', task.id)"
    />
    <button type="button" class="body" @click="emit('open', task.id)">
      <span class="title">
        <span
          v-if="task.priorityFlag === 'high'"
          class="priority"
          role="img"
          aria-label="High priority"
          ><UiIcon name="warning" :size="20"
        /></span>
        {{ task.title }}
      </span>
      <span class="meta">
        <span v-if="due" class="due" :class="{ late: overdue }">{{ due }}</span>
        <span v-if="task.stepsTotal > 0" class="steps"
          >{{ task.stepsDone }}/{{ task.stepsTotal }} steps</span
        >
      </span>
    </button>
    <span v-if="task.assignees.length" class="people">
      <Avatar
        v-for="m in task.assignees"
        :key="m.id"
        :name="m.name"
        :color="m.color"
        :src="m.avatarUrl"
        size="1.75rem"
      />
    </span>
    <span class="claim">
      <slot name="claim">
        <UiButton
          v-if="!task.claimedBy && task.status === 'open' && task.assignees.length === 0"
          variant="tonal"
          class="claim-btn"
          @click="emit('claim', task.id)"
        >
          Claim
        </UiButton>
      </slot>
    </span>
  </div>
</template>

<style scoped>
.task-item {
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
  min-height: var(--ui-touch);
  padding: var(--ui-space-2) var(--ui-space-3);
  color: var(--ui-on-surface);
}
.check {
  flex: none;
  width: 1.5rem;
  height: 1.5rem;
  accent-color: var(--ui-primary);
}
.body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--ui-space-1);
  text-align: left;
  background: none;
  border: 0;
  padding: 0;
  color: inherit;
  font: var(--ui-type-body-lg);
  cursor: pointer;
}
.title {
  font: var(--ui-type-title-md);
  overflow-wrap: anywhere;
}
.priority {
  display: inline-flex;
  vertical-align: middle;
  margin-right: var(--ui-space-1);
  color: var(--ui-error);
}
.meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ui-space-2);
  font: var(--ui-type-body-sm);
  color: var(--ui-on-surface-variant);
}
.due.late {
  color: var(--ui-error);
  font-weight: 600;
}
.people {
  display: inline-flex;
  gap: var(--ui-space-1);
}
.done .title {
  text-decoration: line-through;
  color: var(--ui-on-surface-variant);
}
.kiosk {
  gap: var(--ui-space-4);
  padding: var(--ui-space-3) var(--ui-space-4);
  min-height: 5rem;
}
.kiosk .check {
  width: 3rem;
  height: 3rem;
}
.kiosk .title {
  font-size: var(--kiosk-font-base);
}
.kiosk .meta {
  font-size: var(--kiosk-font-small);
}
.kiosk .claim-btn {
  min-height: 4rem;
  font-size: var(--kiosk-font-small);
}
</style>
