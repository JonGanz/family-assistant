<script setup lang="ts">
import { computed } from 'vue'
import { S } from '../strings'
import { sortedMeals } from '../mealsModel'
import type { PanelProps } from '../types'

const props = defineProps<PanelProps>()
const meals = computed(() => sortedMeals(props.data))
</script>

<template>
  <section class="meals" :aria-label="S.mealsToday" data-testid="panel-meals">
    <h2>{{ S.mealsToday }}</h2>
    <p v-if="meals.length === 0" class="none">{{ S.noMeals }}</p>
    <ul v-else>
      <li v-for="(m, i) in meals" :key="i" data-testid="meal-row">
        <span class="slot">{{ m.slot_name }}</span>
        <span class="time">{{ m.time }}</span>
        <span class="title">{{ m.title }}</span>
      </li>
    </ul>
  </section>
</template>

<style scoped>
h2 {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-lg);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}
li {
  display: grid;
  grid-template-columns: auto auto 1fr;
  align-items: baseline;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-left: 6px solid var(--color-accent);
  border-radius: var(--ui-radius-sm);
  background: var(--color-layer-2, var(--color-surface));
  font-size: var(--font-size-lg);
}
.slot {
  font-weight: 700;
}
.time {
  color: var(--color-muted);
  font-size: var(--font-size-sm);
}
.title {
  min-width: 0;
  overflow-wrap: anywhere;
}
.none {
  margin: 0;
  color: var(--color-muted);
}
</style>
