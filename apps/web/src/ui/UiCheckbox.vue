<script setup lang="ts">
defineProps<{ modelValue: boolean; label?: string; disabled?: boolean; indeterminate?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
</script>

<template>
  <label class="ui-check" :class="{ 'is-disabled': disabled }">
    <input
      type="checkbox"
      :checked="modelValue"
      :indeterminate="indeterminate"
      :disabled="disabled"
      @change="emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    />
    <span
      ><slot>{{ label }}</slot></span
    >
  </label>
</template>

<style scoped>
.ui-check {
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
  min-height: var(--ui-touch);
  font: var(--ui-type-body-lg);
  color: var(--ui-on-surface);
  cursor: pointer;
}
.ui-check input {
  flex: none;
  width: 20px;
  height: 20px;
  margin: 0;
  accent-color: var(--ui-primary);
  cursor: inherit;
}
.ui-check input:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
.ui-check.is-disabled {
  opacity: 0.38;
  cursor: not-allowed;
}
</style>
