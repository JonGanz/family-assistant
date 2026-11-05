<script setup lang="ts">
defineProps<{ modelValue: boolean; label?: string; disabled?: boolean; loading?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
</script>

<template>
  <button
    class="ui-switch"
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :aria-busy="loading || undefined"
    :disabled="disabled"
    @click="emit('update:modelValue', !modelValue)"
  >
    <span v-if="label || $slots.default" class="ui-switch__label"
      ><slot>{{ label }}</slot></span
    >
    <span class="ui-switch__track" :class="{ 'is-on': modelValue }">
      <span class="ui-switch__thumb" />
    </span>
  </button>
</template>

<style scoped>
.ui-switch {
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ui-space-4);
  min-height: var(--ui-touch);
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--ui-on-surface);
  font: var(--ui-type-body-lg);
  text-align: left;
  cursor: pointer;
}
.ui-switch__label {
  flex: 1;
}
.ui-switch__track {
  position: relative;
  flex: none;
  width: 52px;
  height: 32px;
  border: 2px solid var(--ui-outline);
  border-radius: var(--ui-radius-full);
  background: var(--ui-surface-container-highest);
  transition: background var(--ui-motion-short) var(--ui-ease);
}
.ui-switch__track.is-on {
  border-color: var(--ui-primary);
  background: var(--ui-primary);
}
.ui-switch__thumb {
  position: absolute;
  top: 50%;
  left: 6px;
  width: 16px;
  height: 16px;
  border-radius: var(--ui-radius-full);
  background: var(--ui-outline);
  transform: translateY(-50%);
  transition:
    transform var(--ui-motion-short) var(--ui-ease),
    width var(--ui-motion-short) var(--ui-ease),
    height var(--ui-motion-short) var(--ui-ease),
    background var(--ui-motion-short) var(--ui-ease);
}
.ui-switch__track.is-on .ui-switch__thumb {
  width: 24px;
  height: 24px;
  background: var(--ui-on-primary);
  transform: translate(18px, -50%);
  left: 2px;
}
.ui-switch:hover .ui-switch__thumb {
  box-shadow: 0 0 0 8px var(--ui-state-hover);
}
.ui-switch:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
  border-radius: var(--ui-radius-sm);
}
.ui-switch:disabled {
  cursor: not-allowed;
  opacity: 0.38;
}
</style>
