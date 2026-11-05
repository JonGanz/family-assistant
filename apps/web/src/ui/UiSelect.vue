<script setup lang="ts">
import { computed, useId } from 'vue'
import UiIcon from './UiIcon.vue'

export interface SelectOption {
  value: string
  label: string
  disabled?: boolean
}
const props = defineProps<{
  modelValue: string
  options: SelectOption[]
  label: string
  supporting?: string
  error?: string
  disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const id = useId()
const descId = computed(() => (props.error || props.supporting ? `${id}-d` : undefined))
</script>

<template>
  <div class="ui-field" :class="{ 'has-error': error }">
    <label :for="id" class="ui-field__label">{{ label }}</label>
    <div class="ui-field__box">
      <select
        :id="id"
        class="ui-field__input"
        :value="modelValue"
        :disabled="disabled"
        :aria-invalid="error ? true : undefined"
        :aria-describedby="descId"
        @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="o in options" :key="o.value" :value="o.value" :disabled="o.disabled">
          {{ o.label }}
        </option>
      </select>
      <UiIcon name="expand" class="ui-field__chev" />
    </div>
    <p v-if="error || supporting" :id="descId" class="ui-field__help">{{ error || supporting }}</p>
  </div>
</template>

<style scoped>
.ui-field {
  display: grid;
  gap: var(--ui-space-1);
  min-width: 0;
}
.ui-field__label {
  font: var(--ui-type-label-lg);
  color: var(--ui-on-surface-variant);
}
.ui-field__box {
  position: relative;
  display: flex;
  align-items: center;
  min-height: var(--ui-touch);
  border: 1px solid var(--ui-outline);
  border-radius: var(--ui-radius-sm);
  background: var(--ui-surface-container-lowest);
  color: var(--ui-on-surface);
}
.ui-field__box:focus-within {
  border-color: var(--ui-primary);
  outline: 1px solid var(--ui-primary);
}
.ui-field__input {
  flex: 1;
  min-width: 0;
  min-height: 46px;
  padding: 0 40px 0 var(--ui-space-4);
  border: 0;
  outline: 0;
  appearance: none;
  background: transparent;
  color: inherit;
  font: var(--ui-type-body-lg);
}
.ui-field__chev {
  position: absolute;
  right: var(--ui-space-3);
  pointer-events: none;
  color: var(--ui-on-surface-variant);
}
.ui-field__help {
  margin: 0;
  padding: 0 var(--ui-space-4);
  font: var(--ui-type-body-sm);
  color: var(--ui-on-surface-variant);
}
.has-error .ui-field__box {
  border-color: var(--ui-error);
}
.has-error .ui-field__help,
.has-error .ui-field__label {
  color: var(--ui-error);
}
</style>
