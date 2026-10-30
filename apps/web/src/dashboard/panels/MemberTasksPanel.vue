<script setup lang="ts">
import { computed } from 'vue'
import Avatar from '../../components/Avatar.vue'
import TaskItem from '../../components/tasks/TaskItem.vue'
import { hueVar } from '../../lib/hues'
import { visibleMembers } from '../filter'
import { S } from '../strings'
import { memberColumns, toTaskItem } from '../tasksModel'
import type { PanelProps } from '../types'

const props = defineProps<PanelProps>()
const emit = defineEmits<{ complete: [id: string] }>()
const tz = computed(() => props.data.household.tz)
const readOnly = computed(() => props.data.completion_enabled === false)
const cols = computed(() =>
  memberColumns(props.data, visibleMembers(props.data, props.settings), props.panel, props.now),
)
</script>

<template>
  <section class="mt" aria-label="Member tasks">
    <div v-for="c in cols" :key="c.key" class="col" :data-member="c.key">
      <h2>
        <Avatar
          v-if="c.member"
          :name="c.member.display_name"
          :color="hueVar(c.member.color)"
          :src="c.member.avatar_url"
          size="1.6em"
        />
        <span>{{ c.member ? c.member.display_name : S.anyone }}</span>
      </h2>
      <p v-if="c.tasks.length === 0" class="none">{{ S.nothingToDo }}</p>
      <ul>
        <li
          v-for="t in c.tasks"
          :key="`${t.id}-${data.epoch ?? 0}`"
          :class="{ ro: readOnly }"
          :inert="readOnly"
        >
          <TaskItem
            :task="toTaskItem(t, data.members)"
            :now="now"
            :tz="tz"
            size="kiosk"
            @complete="(id: string) => emit('complete', id)"
          />
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.mt {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(0, 1fr);
  gap: var(--space-3);
  align-content: start;
}
.col {
  display: grid;
  gap: var(--space-2);
  align-content: start;
  min-width: 0;
}
h2 {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: 600;
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
.ro :deep(.check) {
  display: none;
}
:deep(.claim) {
  display: none;
}
</style>
