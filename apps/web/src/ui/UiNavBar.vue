<script setup lang="ts">
import { RouterLink } from 'vue-router'
import UiBadge from './UiBadge.vue'
import UiIcon from './UiIcon.vue'
import type { NavDestination } from './nav'
import { useActivePath } from './navItem'

const props = defineProps<{ destinations: NavDestination[]; label?: string; activePath?: string }>()
const active = useActivePath(() => props.activePath)
</script>

<template>
  <nav class="ui-navbar" :aria-label="label ?? 'Primary'">
    <RouterLink
      v-for="d in destinations"
      :key="d.id"
      :to="d.to"
      class="ui-navbar__item"
      :class="{ 'is-active': active(d.to) }"
      :aria-current="active(d.to) ? 'page' : undefined"
    >
      <span class="ui-navbar__pill">
        <UiIcon :name="d.icon" />
        <UiBadge v-if="d.badge" :count="d.badge" class="ui-navbar__badge" />
      </span>
      <span class="ui-navbar__label">{{ d.label }}</span>
    </RouterLink>
  </nav>
</template>

<style scoped>
.ui-navbar {
  display: flex;
  min-height: 80px;
  padding-bottom: env(safe-area-inset-bottom);
  background: var(--ui-surface-container-low);
  color: var(--ui-on-surface-variant);
}
.ui-navbar__item {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--ui-space-1);
  min-width: 0;
  padding: var(--ui-space-3) 0 var(--ui-space-4);
  color: inherit;
  text-decoration: none;
}
.ui-navbar__pill {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 64px;
  height: 32px;
  border-radius: var(--ui-radius-full);
  transition: background var(--ui-motion-short) var(--ui-ease);
}
.ui-navbar__item:hover .ui-navbar__pill {
  background: var(--ui-state-hover);
}
.ui-navbar__item.is-active {
  color: var(--ui-on-surface);
}
.ui-navbar__item.is-active .ui-navbar__pill {
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.ui-navbar__badge {
  position: absolute;
  top: 0;
  left: 50%;
  margin-left: 4px;
}
.ui-navbar__label {
  max-width: 100%;
  overflow: hidden;
  font: var(--ui-type-label-md);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ui-navbar__item.is-active .ui-navbar__label {
  font-weight: 700;
}
.ui-navbar__item:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: -4px;
}
</style>
