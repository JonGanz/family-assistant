<script setup lang="ts">
import { computed } from 'vue'
import MemberChip from '../../components/MemberChip.vue'
import { stripeStyle } from '../../lib/eventColors'
import { dayKey, formatDayHeading, formatRange, groupByDay } from '../../lib/time'
import { toEventLike, toMemberLike, visibleEvents } from '../filter'
import { S } from '../strings'
import { datedTasks } from '../tasksModel'
import type { DetailRequest, PanelProps } from '../types'

const props = defineProps<PanelProps>()
const emit = defineEmits<{ detail: [d: DetailRequest] }>()
const chipsOf = (ids: string[]) => members.value.filter((m) => ids.includes(m.id))
const tz = computed(() => props.data.household.tz)
const today = computed(() => dayKey(props.now, tz.value))
const members = computed(() => props.data.members.map(toMemberLike))
const keyDates = computed(() => datedTasks(props.data, tz.value, false))
const groups = computed(() => {
  const events = visibleEvents(props.data, props.settings).map((e) => toEventLike(e, members.value))
  const days = Math.max(1, props.settings.agenda_days)
  return [...groupByDay(events, tz.value, today.value, days)].map(([key, list]) => ({
    key,
    heading: formatDayHeading(key, tz.value, today.value),
    events: list,
    keyDates: keyDates.value.get(key) ?? [],
  }))
})
</script>

<template>
  <section class="agenda" aria-label="Agenda">
    <div v-for="g in groups" :key="g.key" class="day" :data-day="g.key">
      <h2>{{ g.heading }}</h2>
      <p v-if="g.events.length === 0 && g.keyDates.length === 0" class="none">
        {{ S.nothingPlanned }}
      </p>
      <div
        v-for="t in g.keyDates"
        :key="t.id"
        class="keydate"
        :class="{ done: t.status === 'done' }"
        data-testid="key-date"
      >
        <span class="flag" aria-hidden="true">⚑</span>
        <span class="kt">{{ t.title }}</span>
        <span class="kl">{{ S.keyDate }}</span>
      </div>
      <button
        v-for="(e, i) in g.events"
        :key="e.id ?? i"
        type="button"
        class="event"
        :class="{ busy: e.visibility === 'busy' }"
        :style="stripeStyle(e)"
        :disabled="!e.id"
        @click="e.id && emit('detail', { kind: 'event', id: e.id })"
      >
        <span class="when">
          <span class="time">{{ e.all_day ? S.allDay : formatRange(e, tz) }}</span>
          <span v-if="chipsOf(e.members).length" class="chips">
            <MemberChip
              v-for="m in chipsOf(e.members)"
              :key="m.id"
              :name="m.display_name"
              :color="m.color"
            />
          </span>
        </span>
        <span class="title">{{ e.visibility === 'busy' ? S.busy : (e.title ?? '') }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.agenda {
  display: grid;
  gap: var(--space-5);
  align-content: start;
}
h2 {
  margin: 0 0 var(--space-1);
  font-size: var(--font-size-lg);
  font-weight: 600;
}
.day {
  display: grid;
  gap: var(--space-3);
}
.keydate {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border: 2px dashed var(--color-accent);
  border-radius: var(--radius-md);
  font-size: var(--font-size-base);
}
.keydate.done .kt {
  text-decoration: line-through;
  color: var(--color-muted);
}
.kl {
  margin-left: auto;
  color: var(--color-muted);
  font-size: var(--font-size-sm);
}
.event {
  /* One fixed time column, so every title starts at the same x. */
  display: grid;
  grid-template-columns: 11em minmax(0, 1fr);
  align-items: start;
  gap: var(--space-3);
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border: 0;
  border-left: 6px solid var(--color-accent);
  background: var(--color-surface);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.event.busy {
  color: var(--color-muted);
}
.when {
  display: grid;
  gap: var(--space-1);
  justify-items: start;
}
.time {
  color: var(--color-muted);
  font-size: var(--font-size-sm);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}
.title {
  font-weight: 600;
  overflow-wrap: anywhere;
}
/* Portrait: time and people on the first row (people right-aligned), the title on the second. */
.portrait .event {
  grid-template-columns: minmax(0, 1fr);
  gap: var(--space-1);
}
.portrait .when {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}
.portrait .chips {
  justify-content: flex-end;
}
.none {
  margin: 0;
  color: var(--color-muted);
  font-size: var(--font-size-base);
}
</style>
