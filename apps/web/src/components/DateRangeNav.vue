<script setup lang="ts">
import type { RangeMode } from '../lib/types'
defineProps<{ label: string; mode?: RangeMode }>()
defineEmits<{ prev: []; next: []; today: []; 'update:mode': [mode: RangeMode] }>()
</script>

<template>
  <nav class="range-nav" aria-label="Date range">
    <button type="button" aria-label="Previous" @click="$emit('prev')">‹</button>
    <button type="button" @click="$emit('today')">Today</button>
    <button type="button" aria-label="Next" @click="$emit('next')">›</button>
    <strong class="label" aria-live="polite">{{ label }}</strong>
    <span v-if="mode" class="modes" role="group" aria-label="View">
      <button
        v-for="m in ['day', 'week', 'month'] as RangeMode[]"
        :key="m"
        type="button"
        :aria-pressed="m === mode"
        @click="$emit('update:mode', m)"
      >
        {{ m }}
      </button>
    </span>
  </nav>
</template>

<style scoped>
.range-nav {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.label {
  margin: 0 var(--space-2);
}
.modes {
  margin-left: auto;
  display: flex;
  gap: var(--space-1);
}
button[aria-pressed='true'] {
  background: var(--color-accent);
  color: var(--color-bg);
}
</style>
