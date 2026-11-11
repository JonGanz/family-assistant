<script setup lang="ts">
import { computed } from 'vue'
import { formatDayHeading } from '../../lib/time'
import type { EventLike, MemberLike } from '../../lib/types'
import { UiButton, UiSheet } from '../../ui'
import EventCard from '../EventCard.vue'

/** A day's full event list ("+N more") with drill-in to one event and a way back; shared by Month and Week. */
export interface DayOpen {
  day: string
  events: EventLike[]
}
const props = defineProps<{
  /** The open day list, or null. */
  day: DayOpen | null
  /** The event shown in detail, or null. */
  detail: EventLike | null
  tz: string
  members: MemberLike[]
  today?: string
}>()
const emit = defineEmits<{
  'update:day': [v: DayOpen | null]
  'update:detail': [v: EventLike | null]
}>()

const heading = computed(() =>
  props.day ? formatDayHeading(props.day.day, props.tz, props.today) : '',
)
const open = computed({
  get: () => props.detail !== null || props.day !== null,
  set: (v) => {
    if (!v) {
      emit('update:detail', null)
      emit('update:day', null)
    }
  },
})
const title = computed(() => {
  if (props.detail) return props.detail.visibility === 'busy' ? 'Busy' : (props.detail.title ?? '')
  return heading.value
})
</script>

<template>
  <UiSheet v-model="open" :title="title">
    <template v-if="detail">
      <UiButton
        v-if="day"
        class="back"
        variant="text"
        icon="back"
        @click="emit('update:detail', null)"
      >
        Back to {{ heading }}
      </UiButton>
      <EventCard :event="detail" :tz="tz" :members="members" />
    </template>
    <div v-else class="day-list">
      <EventCard
        v-for="e in day?.events ?? []"
        :key="e.id"
        :event="e"
        :tz="tz"
        :members="members"
        selectable
        @select="emit('update:detail', $event)"
      />
    </div>
  </UiSheet>
</template>

<style scoped>
.day-list {
  display: grid;
  gap: var(--ui-space-2);
}
.back {
  margin-bottom: var(--ui-space-2);
}
</style>
