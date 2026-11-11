<script setup lang="ts">
import { UiButton, UiIconButton, UiSegmented } from '../ui'
import type { RangeMode } from '../lib/types'

defineProps<{ label: string; mode?: RangeMode }>()
defineEmits<{ prev: []; next: []; today: []; 'update:mode': [mode: RangeMode] }>()
const MODES = [
  { value: 'day', label: 'Day' },
  { value: 'week', label: 'Week' },
  { value: 'month', label: 'Month' },
]
</script>

<template>
  <nav class="range-nav" aria-label="Date range">
    <UiIconButton icon="chevron_left" label="Previous" @click="$emit('prev')" />
    <UiIconButton icon="chevron_right" label="Next" @click="$emit('next')" />
    <UiButton variant="tonal" @click="$emit('today')">Today</UiButton>
    <strong class="label" aria-live="polite">{{ label }}</strong>
    <UiSegmented
      v-if="mode"
      class="modes"
      :model-value="mode"
      :options="MODES"
      label="View"
      @update:model-value="$emit('update:mode', $event as RangeMode)"
    />
  </nav>
</template>

<style scoped>
.range-nav {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.label {
  margin: 0 var(--ui-space-2);
  font: var(--ui-type-title-md);
  flex: 1 1 auto;
}
.modes {
  margin-left: auto;
}
</style>
