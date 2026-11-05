<script setup lang="ts">
import UiIcon from './UiIcon.vue'
import type { IconName } from './icons'

defineProps<{ icon: IconName; label: string; extended?: boolean }>()
</script>

<template>
  <button
    class="ui-fab"
    :class="{ 'ui-fab--extended': extended }"
    type="button"
    :aria-label="extended ? undefined : label"
  >
    <UiIcon :name="icon" />
    <span v-if="extended">{{ label }}</span>
  </button>
</template>

<style scoped>
.ui-fab {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--ui-space-3);
  min-width: 56px;
  height: 56px;
  padding: 0 var(--ui-space-4);
  border: 0;
  border-radius: var(--ui-radius-lg);
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
  box-shadow: var(--ui-elev-3);
  font: var(--ui-type-label-lg);
  cursor: pointer;
  isolation: isolate;
}
.ui-fab--extended {
  padding: 0 var(--ui-space-5) 0 var(--ui-space-4);
}
.ui-fab::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  z-index: -1;
}
.ui-fab:hover::before {
  background: var(--ui-state-hover);
}
.ui-fab:focus-visible::before,
.ui-fab:active::before {
  background: var(--ui-state-pressed);
}
.ui-fab:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
</style>
