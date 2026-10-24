<script setup lang="ts">
import { computed } from 'vue'
import EventCard from '../../components/EventCard.vue'
import { dayKey, formatDayHeading, groupByDay } from '../../lib/time'
import { toEventLike, toMemberLike, visibleEvents } from '../filter'
import { S } from '../strings'
import type { PanelProps } from '../types'

const props = defineProps<PanelProps>()
const tz = computed(() => props.data.household.tz)
const today = computed(() => dayKey(props.now, tz.value))
const members = computed(() => props.data.members.map(toMemberLike))
const groups = computed(() => {
  const events = visibleEvents(props.data, props.settings).map(toEventLike)
  const days = Math.max(1, props.settings.agenda_days)
  return [...groupByDay(events, tz.value, today.value, days)].map(([key, list]) => ({
    key,
    heading: formatDayHeading(key, tz.value, today.value),
    events: list,
  }))
})
</script>

<template>
  <section class="agenda" aria-label="Agenda">
    <div v-for="g in groups" :key="g.key" class="day" :data-day="g.key">
      <h2>{{ g.heading }}</h2>
      <p v-if="g.events.length === 0" class="none">{{ S.nothingPlanned }}</p>
      <EventCard
        v-for="(e, i) in g.events"
        :key="e.id ?? i"
        :event="e"
        :tz="tz"
        :members="members"
      />
    </div>
  </section>
</template>

<style scoped>
.agenda {
  display: grid;
  gap: var(--space-3);
  align-content: start;
}
h2 {
  margin: 0 0 var(--space-1);
  font-size: var(--font-size-lg);
  font-weight: 600;
}
.day {
  display: grid;
  gap: var(--space-2);
}
.none {
  margin: 0;
  color: var(--color-muted);
  font-size: var(--font-size-base);
}
</style>
