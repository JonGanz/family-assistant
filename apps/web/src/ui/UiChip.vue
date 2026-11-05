<script setup lang="ts">
import UiIcon from './UiIcon.vue'
import type { IconName } from './icons'

withDefaults(
  defineProps<{
    variant?: 'assist' | 'filter' | 'input'
    selected?: boolean
    icon?: IconName
    disabled?: boolean
  }>(),
  { variant: 'assist' },
)
defineEmits<{ click: [e: MouseEvent]; remove: [] }>()
</script>

<template>
  <span class="ui-chip-wrap" :class="{ 'is-input': variant === 'input' }">
    <button
      class="ui-chip"
      :class="{ 'is-selected': selected }"
      type="button"
      :role="variant === 'filter' ? 'checkbox' : undefined"
      :aria-checked="variant === 'filter' ? !!selected : undefined"
      :disabled="disabled"
      @click="$emit('click', $event)"
    >
      <UiIcon v-if="variant === 'filter' && selected" name="check" :size="20" />
      <UiIcon v-else-if="icon" :name="icon" :size="20" />
      <slot />
    </button>
    <button
      v-if="variant === 'input'"
      class="ui-chip__x"
      type="button"
      aria-label="Remove"
      :disabled="disabled"
      @click="$emit('remove')"
    >
      <UiIcon name="close" :size="20" />
    </button>
  </span>
</template>

<style scoped>
.ui-chip-wrap {
  position: relative;
  display: inline-flex;
  align-items: center;
  height: 32px;
  margin: 8px 0;
}
.ui-chip {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: var(--ui-space-2);
  height: 32px;
  padding: 0 var(--ui-space-3);
  border: 1px solid var(--ui-outline);
  border-radius: var(--ui-radius-sm);
  background: transparent;
  color: var(--ui-on-surface);
  font: var(--ui-type-label-lg);
  cursor: pointer;
}
.ui-chip::after {
  content: '';
  position: absolute;
  inset: -8px 0;
}
.is-input .ui-chip {
  padding-right: 36px;
}
.ui-chip:hover {
  background: var(--ui-state-hover);
}
.ui-chip.is-selected {
  border-color: transparent;
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.ui-chip:focus-visible,
.ui-chip__x:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
.ui-chip__x {
  position: absolute;
  right: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 0;
  border-radius: var(--ui-radius-sm);
  background: transparent;
  color: inherit;
  cursor: pointer;
}
.ui-chip:disabled,
.ui-chip__x:disabled {
  opacity: 0.38;
  cursor: not-allowed;
}
</style>
