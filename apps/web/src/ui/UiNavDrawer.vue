<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import UiBadge from './UiBadge.vue'
import UiIcon from './UiIcon.vue'
import type { NavDestination } from './nav'
import { useActivePath } from './navItem'

const props = defineProps<{ destinations: NavDestination[]; label?: string; activePath?: string }>()
const active = useActivePath(() => props.activePath)
const groups = computed(() => {
  const out: { name: string; items: NavDestination[] }[] = []
  for (const d of props.destinations) {
    const name = d.group ?? ''
    let g = out.find((x) => x.name === name)
    if (!g) out.push((g = { name, items: [] }))
    g.items.push(d)
  }
  return out
})
</script>

<template>
  <nav class="ui-drawer" :aria-label="label ?? 'Primary'">
    <section v-for="g in groups" :key="g.name">
      <h2 v-if="g.name" class="ui-drawer__group">{{ g.name }}</h2>
      <RouterLink
        v-for="d in g.items"
        :key="d.id"
        :to="d.to"
        class="ui-drawer__item"
        :class="{ 'is-active': active(d.to) }"
        :aria-current="active(d.to) ? 'page' : undefined"
      >
        <UiIcon :name="d.icon" />
        <span class="ui-drawer__label">{{ d.label }}</span>
        <UiBadge v-if="d.badge" :count="d.badge" />
      </RouterLink>
    </section>
  </nav>
</template>

<style scoped>
.ui-drawer {
  display: flex;
  flex-direction: column;
  gap: var(--ui-space-3);
  width: 296px;
  padding: var(--ui-space-4) var(--ui-space-3);
  border-radius: 0 var(--ui-radius-xl) var(--ui-radius-xl) 0;
  background: var(--ui-surface-container-low);
  color: var(--ui-on-surface-variant);
}
.ui-drawer__group {
  margin: 0;
  padding: var(--ui-space-2) var(--ui-space-4);
  font: var(--ui-type-title-sm);
  color: var(--ui-on-surface-variant);
}
.ui-drawer__item {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
  min-height: 56px;
  padding: 0 var(--ui-space-4);
  border-radius: var(--ui-radius-full);
  color: inherit;
  font: var(--ui-type-label-lg);
  text-decoration: none;
}
.ui-drawer__label {
  flex: 1;
}
.ui-drawer__item:hover {
  background: var(--ui-state-hover);
}
.ui-drawer__item.is-active {
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.ui-drawer__item:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
</style>
