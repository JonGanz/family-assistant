<script setup lang="ts">
import { S } from '../strings'
import type { PanelProps } from '../types'

defineProps<PanelProps>()
defineEmits<{ dismiss: [id: string] }>()
</script>

<template>
  <section class="advisories" aria-label="Advisories">
    <p v-if="data.advisories.length === 0" class="empty">{{ S.noAdvisories }}</p>
    <ul v-else>
      <li
        v-for="a in data.advisories"
        :key="a.id"
        :class="a.severity"
        :data-severity="a.severity"
        role="status"
      >
        <span class="text">{{ a.text }}</span>
        <button
          type="button"
          :aria-label="`${S.dismiss}: ${a.text}`"
          @click="$emit('dismiss', a.id)"
        >
          ✕
        </button>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.empty {
  margin: 0;
  color: var(--color-muted);
  font-size: var(--font-size-base);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}
li {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-md);
  border-left: 8px solid var(--sev);
  background: var(--color-surface);
  font-size: var(--font-size-base);
  --sev: var(--color-accent);
}
li.warning {
  --sev: var(--color-warning);
}
li.severe {
  --sev: var(--color-danger);
  font-weight: 600;
}
.text {
  flex: 1;
}
button {
  min-width: var(--min-touch, 48px);
  min-height: var(--min-touch, 48px);
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  color: inherit;
  font-size: inherit;
  cursor: pointer;
}
</style>
