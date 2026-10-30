<script setup lang="ts">
import { computed } from 'vue'
import { formatRange } from '../../lib/time'
import Modal from '../Modal.vue'
import { S } from '../strings'
import type { DashboardData } from '../types'

const props = defineProps<{
  data: DashboardData
  title: string
  color: string
  events: string[]
  tasks: string[]
}>()
const emit = defineEmits<{ close: []; event: [id: string] }>()
const tz = computed(() => props.data.household.tz)
const evs = computed(() =>
  props.events
    .map((id) => props.data.events.find((e) => e.id === id))
    .filter((e): e is NonNullable<typeof e> => !!e)
    .sort((a, b) => Number(b.all_day) - Number(a.all_day) || a.start_at.localeCompare(b.start_at)),
)
const tks = computed(() =>
  props.tasks
    .map((id) => (props.data.tasks ?? []).find((t) => t.id === id))
    .filter((t): t is NonNullable<typeof t> => !!t),
)
</script>

<template>
  <Modal :title="title" @close="emit('close')">
    <p v-if="!evs.length && !tks.length" class="none">{{ S.noItems }}</p>
    <ul>
      <li
        v-for="e in evs"
        :key="e.id"
        class="event"
        :class="{ busy: e.visibility === 'busy' }"
        :style="{ borderLeftColor: color }"
        role="button"
        tabindex="0"
        @click="emit('event', e.id)"
        @keydown.enter="emit('event', e.id)"
      >
        <div class="title">{{ e.visibility === 'busy' ? S.busy : e.title }}</div>
        <div class="time">
          {{ e.all_day ? S.allDay : formatRange({ start_at: e.start_at, end_at: e.end_at }, tz) }}
        </div>
      </li>
      <li v-for="t in tks" :key="t.id" class="chip" :class="{ done: t.status === 'done' }">
        ⚑ {{ t.title }}
      </li>
    </ul>
  </Modal>
</template>

<style scoped>
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}
li {
  padding: var(--space-2) var(--space-3);
}
li.event {
  border-left: 6px solid var(--color-accent);
  background: var(--color-surface);
  cursor: pointer;
}
li.event.busy {
  color: var(--color-muted);
}
.title {
  font-weight: 700;
}
.time,
.none {
  color: var(--color-muted);
}
li.chip {
  border: 2px dashed var(--color-accent);
}
li.chip.done {
  text-decoration: line-through;
  color: var(--color-muted);
}
</style>
