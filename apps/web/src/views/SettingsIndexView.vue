<script setup lang="ts">
import { computed } from 'vue'
import PageHeader from '../components/PageHeader.vue'
import { useSessionStore } from '../stores/session'
import { visibleGroups } from './settingsNav'

const session = useSessionStore()
const groups = computed(() => visibleGroups(session.isAdult))
</script>

<template>
  <section aria-labelledby="page-title">
    <PageHeader title="Settings" />
    <section v-for="g in groups" :key="g.title" class="group" :aria-label="g.title">
      <h2>{{ g.title }}</h2>
      <ul class="cards">
        <li v-for="p in g.pages" :key="p.to" class="card">
          <RouterLink :to="p.to">{{ p.label }}</RouterLink>
          <p class="muted">{{ p.blurb }}</p>
        </li>
      </ul>
    </section>
  </section>
</template>

<style scoped>
.group h2 {
  font-size: var(--font-size-lg);
  margin: var(--space-4) 0 var(--space-2);
}
.cards {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
  gap: var(--space-3);
}
.cards a {
  font-weight: 600;
}
.cards p {
  margin: var(--space-1) 0 0;
}
</style>
