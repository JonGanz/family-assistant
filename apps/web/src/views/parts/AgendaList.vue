<script setup lang="ts">
import { computed } from 'vue'
import type { EventView } from '../../api/client'
import EventCard from '../../components/EventCard.vue'
import { dayKey, formatDayHeading, groupByDay } from '../../lib/time'
import type { MemberLike } from '../../lib/types'
import { toEventLike } from '../shared'

const props = defineProps<{
  events: EventView[]
  tz: string
  /** First day key (YYYY-MM-DD) and number of days to list. */
  from: string
  days: number
  members: MemberLike[]
  hideEmpty?: boolean
}>()

const grouped = computed(() => {
  const likes = props.events.map((e) => toEventLike(e, props.members))
  const groups = groupByDay(likes, props.tz, props.from, props.days)
  const today = dayKey(new Date(), props.tz)
  return [...groups.entries()]
    .filter(([, list]) => !props.hideEmpty || list.length > 0)
    .map(([key, list]) => ({ key, heading: formatDayHeading(key, props.tz, today), list }))
})
</script>

<template>
  <div class="agenda">
    <section v-for="g in grouped" :key="g.key" :data-day="g.key">
      <h3>{{ g.heading }}</h3>
      <p v-if="!g.list.length" class="empty">Nothing planned.</p>
      <div v-else class="list">
        <EventCard v-for="e in g.list" :key="e.id" :event="e" :tz="tz" :members="members" />
      </div>
    </section>
  </div>
</template>

<style scoped>
h3 {
  margin: var(--space-4) 0 var(--space-2);
}
.list {
  display: grid;
  gap: var(--space-2);
}
.empty {
  color: var(--color-muted);
  margin: 0;
}
</style>
