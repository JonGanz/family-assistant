<script setup lang="ts">
import { computed } from 'vue'
import MemberChip from '../MemberChip.vue'
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
          >!</span
        >{{ task.title }}
      </span>
      <span class="meta">
        <span v-if="due" class="due" :class="{ late: overdue }">{{ due }}</span>
        <span v-if="task.stepsTotal > 0" class="steps"
          >{{ task.stepsDone }}/{{ task.stepsTotal }} steps</span
        >
        <MemberChip v-for="m in task.assignees" :key="m.id" :name="m.name" :color="m.color" />
      </span>
    </button>
    <span class="claim">
      <slot name="claim">
        <button
          v-if="!task.claimedBy && task.status === 'open' && task.assignees.length === 0"
          type="button"
          class="claim-btn"
          @click="emit('claim', task.id)"
        >
          Claim
        </button>
      </slot>
    </span>
  </div>
</template>

<style scoped>
.task-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-height: var(--min-touch, 48px);
  padding: var(--space-2) var(--space-3);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}
.check {
  flex: none;
  width: 1.6rem;
  height: 1.6rem;
  accent-color: var(--color-success);
}
.body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  text-align: left;
  background: none;
  border: 0;
  padding: 0;
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.title {
  font-size: var(--font-size-base);
  font-weight: 500;
}
.priority {
  display: inline-block;
  margin-right: var(--space-2);
  padding: 0 0.45em;
  border-radius: 999px;
  background: var(--color-danger);
  color: var(--color-bg);
  font-weight: 700;
}
.meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-size-sm);
  color: var(--color-muted);
}
.due.late {
  color: var(--color-danger);
  font-weight: 600;
}
.done .title {
  text-decoration: line-through;
  color: var(--color-muted);
}
.claim-btn {
  min-height: var(--min-touch, 48px);
  padding: 0 var(--space-3);
  border: 1px solid var(--color-accent);
  border-radius: var(--radius-md);
  background: none;
  color: var(--color-accent);
  font: inherit;
  cursor: pointer;
}
.kiosk {
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
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
