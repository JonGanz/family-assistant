<script setup lang="ts">
import { computed } from 'vue'
import { textOn } from '../lib/hues'

/** `kiosk` (default) keeps the wall-display look; the app passes `variant="app"`. */
const props = withDefaults(
  defineProps<{ name: string; color: string; variant?: 'kiosk' | 'app' }>(),
  { variant: 'kiosk' },
)
const fg = computed(() => textOn(props.color))
</script>

<template>
  <span
    class="member-chip"
    :class="{ 'member-chip--app': variant === 'app' }"
    :style="{ background: color, color: fg }"
    >{{ name }}</span
  >
</template>

<style scoped>
.member-chip {
  display: inline-block;
  padding: 0 var(--space-2);
  border-radius: 0;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  white-space: nowrap;
}
.member-chip--app {
  padding: 0 var(--ui-space-2);
  border-radius: var(--ui-radius-full);
  font: var(--ui-type-label-md);
  line-height: 1.5rem;
}
</style>
