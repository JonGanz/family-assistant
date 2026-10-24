<script setup lang="ts">
import { computed } from 'vue'
import { formatRange } from '../lib/time'
import type { EventLike, MemberLike } from '../lib/types'
import MemberChip from './MemberChip.vue'

const props = defineProps<{
  event: EventLike
  tz: string
  members?: MemberLike[]
  locale?: string
}>()

const busy = computed(() => props.event.visibility === 'busy')
const chips = computed(() =>
  (props.members ?? []).filter((m) => props.event.members.includes(m.id)),
)
</script>

<template>
  <article class="event-card" :class="{ busy }" :style="{ borderLeftColor: event.color }">
    <div class="time">{{ formatRange(event, tz, locale) }}</div>
    <div class="body">
      <div class="title">{{ busy ? 'Busy' : (event.title ?? '') }}</div>
      <div v-if="!busy && event.location" class="location">{{ event.location }}</div>
      <div v-if="chips.length" class="chips">
        <MemberChip v-for="m in chips" :key="m.id" :name="m.display_name" :color="m.color" />
      </div>
      <a
        v-if="event.source_url"
        class="from"
        :href="event.source_url"
        target="_blank"
        rel="noopener"
        >from ↗</a
      >
    </div>
  </article>
</template>

<style scoped>
.event-card {
  display: flex;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-left: 6px solid var(--color-accent);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}
.event-card.busy {
  background: repeating-linear-gradient(
    45deg,
    var(--color-surface),
    var(--color-surface) 8px,
    var(--color-border) 8px,
    var(--color-border) 10px
  );
  color: var(--color-muted);
}
.time {
  min-width: 7em;
  color: var(--color-muted);
  font-size: var(--font-size-sm);
}
.title {
  font-weight: 600;
}
.location,
.from {
  font-size: var(--font-size-sm);
  color: var(--color-muted);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
  margin-top: var(--space-1);
}
</style>
