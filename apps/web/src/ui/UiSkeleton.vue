<script setup lang="ts">
withDefaults(defineProps<{ lines?: number; shape?: 'text' | 'circle' | 'block' }>(), {
  lines: 1,
  shape: 'text',
})
</script>

<template>
  <div class="ui-skeleton" aria-hidden="true">
    <template v-if="shape === 'text'">
      <span
        v-for="n in lines"
        :key="n"
        class="bar"
        :style="{ width: n === lines && lines > 1 ? '60%' : '100%' }"
      />
    </template>
    <span v-else class="bar" :class="shape" />
  </div>
</template>

<style scoped>
.ui-skeleton {
  display: grid;
  gap: var(--ui-space-2);
}
.bar {
  display: block;
  height: 14px;
  border-radius: var(--ui-radius-xs);
  background: var(--ui-surface-container-high);
  animation: ui-pulse 1.4s ease-in-out infinite;
}
.circle {
  width: 40px;
  height: 40px;
  border-radius: var(--ui-radius-full);
}
.block {
  height: 96px;
  border-radius: var(--ui-radius-md);
}
@keyframes ui-pulse {
  50% {
    opacity: 0.5;
  }
}
@media (prefers-reduced-motion: reduce) {
  .bar {
    animation: none;
  }
}
</style>
