<script setup lang="ts">
import { computed } from 'vue'
import { formatTime } from '../../lib/time'
import type { PanelProps } from '../types'

const props = defineProps<PanelProps>()
const tz = computed(() => props.data.household.tz)
const time = computed(() => formatTime(props.now, tz.value))
const date = computed(() =>
  new Intl.DateTimeFormat(undefined, {
    timeZone: tz.value,
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  }).format(props.now),
)
</script>

<template>
  <section class="clock" aria-label="Clock">
    <div class="time" data-testid="clock">{{ time }}</div>
    <div class="date">{{ date }}</div>
    <div class="household" data-testid="household">{{ data.household.name }}</div>
  </section>
</template>

<style scoped>
.time {
  font-size: 3.2em;
  line-height: 1.05;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
.date {
  font-size: var(--font-size-lg);
}
.household {
  font-size: var(--font-size-base);
  color: var(--color-muted);
}
</style>
