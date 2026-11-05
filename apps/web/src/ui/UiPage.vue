<script setup lang="ts">
defineProps<{ wide?: boolean }>()
</script>

<template>
  <div class="ui-page">
    <slot name="top" />
    <main class="ui-page__body" :class="{ 'is-wide': wide }"><slot /></main>
  </div>
</template>

<style scoped>
.ui-page {
  min-height: 100%;
  background: var(--ui-surface);
  color: var(--ui-on-surface);
  padding-left: env(safe-area-inset-left);
  padding-right: env(safe-area-inset-right);
}
.ui-page__body {
  display: grid;
  gap: var(--ui-space-4);
  max-width: 840px;
  margin: 0 auto;
  padding: var(--ui-space-4);
  padding-bottom: calc(var(--ui-space-4) + env(safe-area-inset-bottom));
}
.ui-page__body.is-wide {
  max-width: none;
}
@media (min-width: 600px) {
  .ui-page__body {
    padding: var(--ui-space-5);
  }
}
</style>
