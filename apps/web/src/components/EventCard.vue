<script setup lang="ts">
import { computed } from 'vue'
import { stripeStyle } from '../lib/eventColors'
import { mapLink } from '../lib/mapLink'
import { formatRange } from '../lib/time'
import type { EventLike, MemberLike } from '../lib/types'
import MemberChip from './MemberChip.vue'

const props = defineProps<{
  event: EventLike
  tz: string
  members?: MemberLike[]
  locale?: string
  /** Makes the card a button that emits `select`. */
  selectable?: boolean
}>()
const emit = defineEmits<{ select: [event: EventLike] }>()

const busy = computed(() => props.event.visibility === 'busy')
const map = computed(() => (props.event.location ? mapLink(props.event.location) : null))
const chips = computed(() =>
  (props.members ?? []).filter((m) => props.event.members.includes(m.id)),
)
</script>

<template>
  <article
    class="event-card"
    :class="{ busy, selectable }"
    :style="stripeStyle(event)"
    :role="selectable ? 'button' : undefined"
    :tabindex="selectable ? 0 : undefined"
    @click="selectable && emit('select', event)"
    @keydown.enter="selectable && emit('select', event)"
    @keydown.space.prevent="selectable && emit('select', event)"
  >
    <div class="time">{{ formatRange(event, tz, locale) }}</div>
    <div class="body">
      <div class="title">{{ busy ? 'Busy' : (event.title ?? '') }}</div>
      <div v-if="!busy && event.location" class="location">
        {{ event.location }}
        <a
          v-if="map"
          class="map-link"
          :href="map.href"
          :target="map.external ? '_blank' : undefined"
          :rel="map.external ? 'noopener' : undefined"
          @click.stop
          >Open in maps</a
        >
      </div>
      <div v-if="chips.length" class="chips">
        <MemberChip
          v-for="m in chips"
          :key="m.id"
          :name="m.display_name"
          :color="m.color"
          variant="app"
        />
      </div>
      <a
        v-if="event.source_url"
        class="from"
        :href="event.source_url"
        target="_blank"
        rel="noopener"
        @click.stop
        >from ↗</a
      >
    </div>
  </article>
</template>

<style scoped>
.event-card {
  display: flex;
  gap: var(--ui-space-3);
  padding: var(--ui-space-3);
  --ev-stripe-w: 6px;
  border-left: 6px solid var(--ui-primary);
  border-radius: var(--ui-radius-md);
  background-color: var(--ui-surface-container-low);
  color: var(--ui-on-surface);
}
.event-card.selectable {
  cursor: pointer;
}
.event-card.selectable:hover {
  background-color: var(--ui-surface-container-high);
  --ui-on-surface-variant: var(--ui-on-surface-variant-high);
}
.event-card.selectable:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
.event-card.busy {
  --ev-bg: repeating-linear-gradient(
    45deg,
    var(--ui-surface-container-low),
    var(--ui-surface-container-low) 8px,
    var(--ui-outline-variant) 8px,
    var(--ui-outline-variant) 10px
  );
  background-image: var(--ev-bg);
  color: var(--ui-on-surface-variant);
}
.time {
  min-width: 6.5em;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.title {
  font: var(--ui-type-title-sm);
}
.location,
.from {
  font: var(--ui-type-body-md);
  color: var(--ui-on-surface-variant);
}
.map-link {
  margin-left: var(--ui-space-2);
  color: var(--ui-primary);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-1);
  margin-top: var(--ui-space-1);
}
</style>
