<script setup lang="ts" generic="T extends { id: string }">
// Rule of the house (R2, 2026-11-05): a collection is a stack of cards on every width. On
// compact the grid collapses to one column. The card content comes from the `item` slot.
import { UiCard } from '../ui'

defineProps<{ items: T[] }>()
</script>

<template>
  <ul class="ri-grid">
    <li v-for="it in items" :key="it.id">
      <UiCard variant="filled"><slot name="item" :item="it" /></UiCard>
    </li>
  </ul>
</template>

<style scoped>
.ri-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--ui-space-3);
  margin: 0;
  padding: 0;
  list-style: none;
}
@media (min-width: 600px) {
  .ri-grid {
    grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
  }
}
</style>
