<script setup lang="ts">
import { computed } from 'vue'
import { UiButton } from '../../ui'
import { S } from '../strings'
import { nowTasks, taskLocked, toTaskItem } from '../tasksModel'
import type { PanelProps } from '../types'
import TaskRow from './TaskRow.vue'

const props = defineProps<PanelProps>()
const emit = defineEmits<{ complete: [id: string]; add: [] }>()
const tz = computed(() => props.data.household.tz)
const items = computed(() =>
  nowTasks(props.data, props.panel).map((t) => ({
    model: toTaskItem(t, props.data.members),
    locked: taskLocked(props.data, t),
  })),
)
const canAdd = computed(() => (props.data.task_lists ?? []).some((l) => l.dashboard_add))
</script>

<template>
  <section class="now" aria-label="Now">
    <header>
      <h2>{{ S.panelNames['now'] }}</h2>
      <UiButton
        v-if="canAdd"
        class="add"
        variant="filled"
        icon="add"
        :aria-label="S.addTask"
        @click="emit('add')"
      >
        {{ S.create }}
      </UiButton>
    </header>
    <p v-if="items.length === 0" class="none">{{ S.nothingToDo }}</p>
    <ul>
      <TaskRow
        v-for="t in items"
        :key="`${t.model.id}-${data.epoch ?? 0}`"
        :task="t.model"
        :locked="t.locked"
        :now="now"
        :tz="tz"
        @complete="(id: string) => emit('complete', id)"
      />
    </ul>
  </section>
</template>

<style scoped>
.now {
  display: grid;
  gap: var(--space-3);
  align-content: start;
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
h2 {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: 600;
}
.add {
  min-height: 4rem;
  padding-inline: var(--space-5);
  font-size: var(--kiosk-font-small);
}
.add :deep(.ui-icon),
.add :deep(svg) {
  width: 2rem;
  height: 2rem;
}
.none {
  margin: 0;
  color: var(--color-muted);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}
</style>
