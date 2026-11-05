<script setup lang="ts" generic="T extends { id: string }">
// Rule of the house: a collection is a UiList on compact and a grid of cards on medium and
// expanded. The row or card content comes from the `item` slot.
import { UiCard, UiList, useBreakpoint } from '../ui'

defineProps<{ items: T[] }>()
const bp = useBreakpoint()
</script>

<template>
  <UiList v-if="bp === 'compact'" class="ri-list">
    <li v-for="it in items" :key="it.id" class="ri-row"><slot name="item" :item="it" /></li>
  </UiList>
  <ul v-else class="ri-grid">
    <li v-for="it in items" :key="it.id">
      <UiCard variant="filled"><slot name="item" :item="it" /></UiCard>
    </li>
  </ul>
</template>

<style scoped>
.ri-list .ri-row + .ri-row {
  border-top: 1px solid var(--ui-outline-variant);
}
.ri-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
  gap: var(--ui-space-3);
  margin: 0;
  padding: 0;
  list-style: none;
}
</style>
