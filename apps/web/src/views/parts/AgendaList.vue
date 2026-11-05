<script setup lang="ts">
import { computed } from 'vue'
import type { EventView } from '../../api/client'
import EventCard from '../../components/EventCard.vue'
import { dayKey, formatDayHeading, groupByDay } from '../../lib/time'
import type { EventLike, MemberLike } from '../../lib/types'
import { UiSection } from '../../ui'
import { toEventLike } from '../shared'

const props = defineProps<{
  events: EventView[]
  tz: string
  /** First day key (YYYY-MM-DD) and number of days to list. */
  from: string
  days: number
  members: MemberLike[]
  hideEmpty?: boolean
  selectable?: boolean
}>()
defineEmits<{ select: [event: EventLike] }>()

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
    <UiSection v-for="g in grouped" :key="g.key" :heading="g.heading" :data-day="g.key">
      <p v-if="!g.list.length" class="empty">Nothing planned.</p>
      <div v-else class="list">
        <EventCard
          v-for="e in g.list"
          :key="e.id"
          :event="e"
          :tz="tz"
          :members="members"
          :selectable="selectable"
          @select="$emit('select', $event)"
        />
      </div>
    </UiSection>
  </div>
</template>

<style scoped>
.agenda {
  display: grid;
  gap: var(--ui-space-4);
}
.list {
  display: grid;
  gap: var(--ui-space-2);
}
.empty {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
  margin: 0;
}
</style>
