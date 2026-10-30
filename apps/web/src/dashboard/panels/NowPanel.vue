<script setup lang="ts">
import { computed } from 'vue'
import TaskItem from '../../components/tasks/TaskItem.vue'
import { S } from '../strings'
import { nowTasks, toTaskItem } from '../tasksModel'
import type { PanelProps } from '../types'

const props = defineProps<PanelProps>()
const emit = defineEmits<{ complete: [id: string]; add: [] }>()
const tz = computed(() => props.data.household.tz)
const readOnly = computed(() => props.data.completion_enabled === false)
const items = computed(() =>
  nowTasks(props.data, props.panel).map((t) => toTaskItem(t, props.data.members)),
)
const canAdd = computed(() => (props.data.task_lists ?? []).some((l) => l.dashboard_add))
</script>

<template>
  <section class="now" aria-label="Now">
    <header>
      <h2>{{ S.panelNames['now'] }}</h2>
      <button v-if="canAdd" type="button" class="add" :aria-label="S.addTask" @click="emit('add')">
        +
      </button>
    </header>
    <p v-if="items.length === 0" class="none">{{ S.nothingToDo }}</p>
    <ul>
      <li
        v-for="t in items"
        :key="`${t.id}-${data.epoch ?? 0}`"
        :class="{ ro: readOnly }"
        :inert="readOnly"
      >
        <TaskItem
          :task="t"
          :now="now"
          :tz="tz"
          size="kiosk"
          @complete="(id: string) => emit('complete', id)"
        />
      </li>
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
  min-width: 4rem;
  min-height: 4rem;
  border: 0;
  border-radius: 0;
  background: var(--color-accent);
  color: var(--color-on-accent);
  font-size: 2.5rem;
  line-height: 1;
  cursor: pointer;
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
.ro :deep(.check),
.ro :deep(.claim) {
  display: none;
}
:deep(.claim) {
  display: none;
}
</style>
