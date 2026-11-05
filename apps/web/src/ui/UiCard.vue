<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'

const props = withDefaults(
  defineProps<{ variant?: 'elevated' | 'filled' | 'outlined'; to?: string }>(),
  { variant: 'filled' },
)
const tag = computed(() => (props.to ? RouterLink : 'div'))
</script>

<template>
  <component :is="tag" :to="to" class="ui-card" :class="[`ui-card--${variant}`, { 'is-link': to }]">
    <div v-if="$slots.media" class="ui-card__media"><slot name="media" /></div>
    <div v-if="$slots.header" class="ui-card__header"><slot name="header" /></div>
    <div v-if="$slots.default" class="ui-card__body"><slot /></div>
    <div v-if="$slots.actions" class="ui-card__actions"><slot name="actions" /></div>
  </component>
</template>

<style scoped>
.ui-card {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  border-radius: var(--ui-radius-md);
  color: var(--ui-on-surface);
  text-decoration: none;
  overflow: hidden;
  isolation: isolate;
}
.ui-card--elevated {
  background: var(--ui-surface-container-low);
  box-shadow: var(--ui-elev-1);
}
.ui-card--filled {
  background: var(--ui-surface-container-high);
}
.ui-card--outlined {
  background: var(--ui-surface);
  border: 1px solid var(--ui-outline-variant);
}
.ui-card__media {
  overflow: hidden;
}
.ui-card__header {
  padding: var(--ui-space-4) var(--ui-space-4) 0;
  font: var(--ui-type-title-md);
}
.ui-card__body {
  padding: var(--ui-space-4);
  font: var(--ui-type-body-md);
  display: grid;
  gap: var(--ui-space-2);
}
.ui-card__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--ui-space-2);
  padding: 0 var(--ui-space-2) var(--ui-space-2);
}
.is-link {
  cursor: pointer;
}
.is-link::before {
  content: '';
  position: absolute;
  inset: 0;
  z-index: 1;
  pointer-events: none;
}
.is-link:hover::before {
  background: var(--ui-state-hover);
}
.is-link:active::before {
  background: var(--ui-state-pressed);
}
.is-link:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
</style>
