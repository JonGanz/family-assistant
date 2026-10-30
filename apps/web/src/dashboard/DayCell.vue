<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { formatRange } from '../lib/time'
import type { EventLike } from '../lib/types'
import { fitCell } from './dayCellFit'
import { S } from './strings'
import type { DetailRequest, TaskView } from './types'

const props = defineProps<{
  events: EventLike[]
  tasks: TaskView[]
  tz: string
  /** Stripe style of the owning member's column. */
  stripe: Record<string, string>
  /** Heading of the "all items" modal, e.g. "Alex, Mon 26". */
  label: string
  color: string
}>()
const emit = defineEmits<{ detail: [d: DetailRequest] }>()

type Item = { kind: 'event'; e: EventLike } | { kind: 'task'; t: TaskView }
/** Events by start time (all-day first), then tasks, so "+N" hides the least time-bound items. */
const items = computed<Item[]>(() => [
  ...[...props.events]
    .sort((a, b) => Number(b.all_day) - Number(a.all_day) || a.start_at.localeCompare(b.start_at))
    .map((e): Item => ({ kind: 'event', e })),
  ...props.tasks.map((t): Item => ({ kind: 'task', t })),
])

const box = ref<HTMLElement | null>(null)
const size = ref({ w: 0, h: 0 })
const remPx = ref(16)
let ro: ResizeObserver | undefined
onMounted(() => {
  const el = box.value
  if (!el) return
  remPx.value = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16
  const read = () => (size.value = { w: el.clientWidth, h: el.clientHeight })
  read()
  if (typeof ResizeObserver !== 'undefined') {
    ro = new ResizeObserver(read)
    ro.observe(el)
  }
})
onBeforeUnmount(() => ro?.disconnect())

const fit = computed(() => fitCell(items.value.length, size.value.w, size.value.h, remPx.value))
const shown = computed(() => items.value.slice(0, fit.value.shown))
const open = (e: EventLike) => e.id && emit('detail', { kind: 'event', id: e.id })
const all = () =>
  emit('detail', {
    kind: 'day',
    title: props.label,
    color: props.color,
    events: props.events.map((e) => e.id).filter((id): id is string => !!id),
    tasks: props.tasks.map((t) => t.id),
  })
</script>

<template>
  <div ref="box" class="fit">
    <ul :style="{ '--n': fit.cols }">
      <template v-for="(it, i) in shown" :key="i">
        <li
          v-if="it.kind === 'event'"
          class="event"
          :class="{ busy: it.e.visibility === 'busy' }"
          :style="stripe"
          :role="it.e.id ? 'button' : undefined"
          :tabindex="it.e.id ? 0 : undefined"
          @click="open(it.e)"
          @keydown.enter="open(it.e)"
        >
          <div class="title">{{ it.e.visibility === 'busy' ? S.busy : it.e.title }}</div>
          <div class="time">{{ it.e.all_day ? S.allDay : formatRange(it.e, tz) }}</div>
        </li>
        <li v-else class="chip" :class="{ done: it.t.status === 'done' }" data-testid="task-chip">
          <div class="title">⚑ {{ it.t.title }}</div>
        </li>
      </template>
      <li v-if="fit.more" class="more">
        <button type="button" :aria-label="S.showAll(fit.more)" @click="all">
          +{{ fit.more }}
        </button>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.fit {
  position: absolute;
  inset: var(--space-1);
  overflow: hidden;
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(var(--n), minmax(0, 1fr));
  grid-auto-rows: 3.2rem;
  gap: 0.25rem;
}
li {
  min-width: 0;
  padding: 0.15rem var(--space-2);
  overflow: hidden;
  font-size: var(--font-size-sm);
  line-height: 1.25;
}
.title,
.time {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.title {
  font-weight: 700;
}
.time {
  color: var(--color-muted);
}
li.event {
  cursor: pointer;
  border-left: 6px solid var(--color-accent);
  background: var(--color-surface);
}
li.event.busy {
  color: var(--color-muted);
}
li.chip {
  display: flex;
  align-items: center;
  border: 2px dashed var(--color-accent);
}
li.chip.done {
  text-decoration: line-through;
  color: var(--color-muted);
}
li.more button {
  width: 100%;
  height: 100%;
  border: 2px solid var(--color-border);
  background: var(--color-bg);
  color: inherit;
  font: inherit;
  font-weight: 700;
  cursor: pointer;
}
</style>
