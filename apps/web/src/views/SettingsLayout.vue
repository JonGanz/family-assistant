<script setup lang="ts">
import { computed } from 'vue'
import { useSessionStore } from '../stores/session'
import { visibleGroups } from './settingsNav'

const session = useSessionStore()
const groups = computed(() => visibleGroups(session.isAdult))
</script>

<template>
  <div class="settings-layout">
    <nav aria-label="Settings" class="settings-nav">
      <RouterLink to="/settings" exact-active-class="router-link-active" class="settings-all"
        >All settings</RouterLink
      >
      <template v-for="g in groups" :key="g.title">
        <h2 class="settings-group">{{ g.title }}</h2>
        <RouterLink v-for="p in g.pages" :key="p.to" :to="p.to">{{ p.label }}</RouterLink>
      </template>
    </nav>
    <div class="settings-body"><RouterView /></div>
  </div>
</template>

<style scoped>
.settings-layout {
  display: grid;
  grid-template-columns: 13rem minmax(0, 1fr);
  gap: var(--space-4);
  align-items: start;
}
.settings-nav {
  display: grid;
  gap: var(--space-1);
  position: sticky;
  top: var(--space-3);
}
.settings-nav a {
  padding: var(--space-1) var(--space-3);
  border-radius: var(--radius-md);
  color: var(--color-text);
  text-decoration: none;
}
.settings-nav a:hover {
  background: var(--color-surface);
}
.settings-nav a.router-link-active {
  color: var(--color-accent);
  font-weight: 600;
  background: var(--color-surface);
}
.settings-group {
  margin: var(--space-3) 0 0;
  padding: 0 var(--space-3);
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--color-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
@media (max-width: 40rem) {
  .settings-layout {
    grid-template-columns: minmax(0, 1fr);
  }
  .settings-nav {
    position: static;
    display: flex;
    gap: var(--space-2);
    overflow-x: auto;
    padding-bottom: var(--space-2);
  }
  .settings-nav a {
    flex: none;
    white-space: nowrap;
    border: 1px solid var(--color-border);
    border-radius: 999px;
  }
  .settings-group {
    display: none;
  }
}
</style>
