<script setup lang="ts">
import { useId } from 'vue'

export interface RadioOption {
  value: string
  label: string
  supporting?: string
  disabled?: boolean
}
defineProps<{ modelValue: string; options: RadioOption[]; legend: string; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const name = useId()
</script>

<template>
  <fieldset class="ui-radios" :disabled="disabled">
    <legend>{{ legend }}</legend>
    <label
      v-for="o in options"
      :key="o.value"
      class="ui-radio"
      :class="{ 'is-disabled': o.disabled }"
    >
      <input
        type="radio"
        :name="name"
        :value="o.value"
        :checked="modelValue === o.value"
        :disabled="o.disabled"
        @change="emit('update:modelValue', o.value)"
      />
      <span>
        {{ o.label }}
        <small v-if="o.supporting">{{ o.supporting }}</small>
      </span>
    </label>
  </fieldset>
</template>

<style scoped>
.ui-radios {
  margin: 0;
  padding: 0;
  border: 0;
  min-width: 0;
}
legend {
  padding: 0;
  font: var(--ui-type-label-lg);
  color: var(--ui-on-surface-variant);
}
.ui-radio {
  display: flex;
  align-items: flex-start;
  gap: var(--ui-space-3);
  min-height: var(--ui-touch);
  padding: 14px 0;
  font: var(--ui-type-body-lg);
  color: var(--ui-on-surface);
  cursor: pointer;
}
.ui-radio input {
  flex: none;
  width: 20px;
  height: 20px;
  margin: 2px 0 0;
  accent-color: var(--ui-primary);
}
.ui-radio input:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
small {
  display: block;
  font: var(--ui-type-body-md);
  color: var(--ui-on-surface-variant);
}
.is-disabled {
  opacity: 0.38;
  cursor: not-allowed;
}
</style>
