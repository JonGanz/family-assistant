<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import UiIcon from './UiIcon.vue'
import UiProgress from './UiProgress.vue'
import type { IconName } from './icons'

const props = withDefaults(
  defineProps<{
    variant?: 'filled' | 'tonal' | 'outlined' | 'text'
    icon?: IconName
    loading?: boolean
    disabled?: boolean
    type?: 'button' | 'submit' | 'reset'
    to?: string
    href?: string
  }>(),
  { variant: 'filled', type: 'button' },
)
const tag = computed(() => (props.to ? RouterLink : props.href ? 'a' : 'button'))
const linkAttrs = computed(() =>
  props.to ? { to: props.to } : props.href ? { href: props.href } : {},
)
</script>

<template>
  <component
    :is="tag"
    class="ui-btn"
    :class="`ui-btn--${variant}`"
    v-bind="linkAttrs"
    :type="tag === 'button' ? type : undefined"
    :disabled="tag === 'button' ? disabled || loading : undefined"
    :aria-busy="loading || undefined"
  >
    <UiProgress v-if="loading" kind="circular" :size="18" label="Working" />
    <UiIcon v-else-if="icon" :name="icon" :size="20" />
    <slot />
  </component>
</template>

<style scoped>
.ui-btn {
  position: relative;
  text-decoration: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--ui-space-2);
  min-height: var(--ui-button-height);
  padding: 0 var(--ui-space-5);
  border: 1px solid transparent;
  border-radius: var(--ui-radius-full);
  font: var(--ui-type-label-lg);
  cursor: pointer;
  overflow: visible;
  isolation: isolate;
  transition: box-shadow var(--ui-motion-short) var(--ui-ease);
}
/* 48 px hit area around the 40 px visual */
.ui-btn::after {
  content: '';
  position: absolute;
  inset: -4px 0;
}
.ui-btn::before {
  content: '';
  position: absolute;
  inset: -1px;
  border-radius: inherit;
  background: transparent;
  pointer-events: none;
  z-index: -1;
}
.ui-btn:hover::before {
  background: var(--ui-state-hover);
}
.ui-btn:focus-visible::before {
  background: var(--ui-state-focus);
}
.ui-btn:active::before {
  background: var(--ui-state-pressed);
}
.ui-btn:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
.ui-btn--filled {
  background: var(--ui-primary);
  color: var(--ui-on-primary);
}
.ui-btn--tonal {
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.ui-btn--outlined {
  border-color: var(--ui-outline);
  color: var(--ui-primary);
  background: transparent;
}
.ui-btn--text {
  color: var(--ui-primary);
  padding: 0 var(--ui-space-4);
}
.ui-btn:disabled {
  cursor: not-allowed;
  background: color-mix(in srgb, var(--ui-on-surface) 12%, transparent);
  color: color-mix(in srgb, var(--ui-on-surface) 38%, transparent);
  border-color: transparent;
}
.ui-btn:disabled::before {
  background: transparent;
}
.ui-btn--outlined:disabled,
.ui-btn--text:disabled {
  background: transparent;
}
.ui-btn--outlined:disabled {
  border-color: color-mix(in srgb, var(--ui-on-surface) 12%, transparent);
}
</style>
