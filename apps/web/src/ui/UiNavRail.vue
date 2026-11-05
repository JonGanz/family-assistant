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
  <nav class="ui-rail" :aria-label="label ?? 'Primary'">
    <RouterLink
      v-for="d in destinations"
      :key="d.id"
      :to="d.to"
      class="ui-rail__item"
      :class="{ 'is-active': active(d.to) }"
      :aria-current="active(d.to) ? 'page' : undefined"
    >
      <span class="ui-rail__pill">
        <UiIcon :name="d.icon" />
        <UiBadge v-if="d.badge" :count="d.badge" class="ui-rail__badge" />
      </span>
      <span class="ui-rail__label">{{ d.label }}</span>
    </RouterLink>
  </nav>
</template>

<style scoped>
.ui-rail {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--ui-space-3);
  width: 80px;
  padding: var(--ui-space-4) 0;
  background: var(--ui-surface-container-low);
  color: var(--ui-on-surface-variant);
}
.ui-rail__item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--ui-space-1);
  width: 100%;
  min-height: var(--ui-touch);
  color: inherit;
  text-decoration: none;
}
.ui-rail__pill {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 56px;
  height: 32px;
  border-radius: var(--ui-radius-full);
}
.ui-rail__item:hover .ui-rail__pill {
  background: var(--ui-state-hover);
}
.ui-rail__item.is-active {
  color: var(--ui-on-surface);
}
.ui-rail__item.is-active .ui-rail__pill {
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.ui-rail__badge {
  position: absolute;
  top: 0;
  left: 50%;
  margin-left: 2px;
}
.ui-rail__label {
  font: var(--ui-type-label-md);
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ui-rail__item:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
</style>
