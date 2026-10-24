<script setup lang="ts">
import type { AdvisoryView } from '../../api/client'

defineProps<{ advisories: AdvisoryView[] }>()
defineEmits<{ dismiss: [id: string] }>()
</script>

<template>
  <ul v-if="advisories.length" class="advisories" aria-label="Advisories">
    <li v-for="a in advisories" :key="a.id" :class="a.severity">
      <span>{{ a.text }}</span>
      <button type="button" aria-label="Dismiss" @click="$emit('dismiss', a.id)">✕</button>
    </li>
  </ul>
</template>

<style scoped>
.advisories {
  list-style: none;
  padding: 0;
  margin: 0 0 var(--space-3);
  display: grid;
  gap: var(--space-2);
}
li {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-left: 6px solid var(--color-accent);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}
li.warning {
  border-left-color: var(--color-warning);
}
li.severe {
  border-left-color: var(--color-danger);
}
button {
  margin-left: auto;
}
</style>
