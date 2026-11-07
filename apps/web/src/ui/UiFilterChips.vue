<script setup lang="ts">
import { computed } from 'vue'
import UiIcon from './UiIcon.vue'
import type { IconName } from './icons'

export interface FilterChipOption {
  value: string
  label: string
  icon?: IconName
  /** A CSS colour: shows a dot before the label (a member's colour). */
  color?: string
}

const props = withDefaults(
  defineProps<{
    options: FilterChipOption[]
    /** The chosen values; empty means All. */
    modelValue: string[]
    allLabel?: string
    /** The group's accessible name. */
    label: string
  }>(),
  { allLabel: 'All' },
)
const emit = defineEmits<{ 'update:modelValue': [value: string[]] }>()

const isAll = computed(() => props.modelValue.length === 0)

/** D-105: All clears the rest; another chip toggles (and clears All); the last one off is All. */
function toggle(value: string) {
  const next = props.modelValue.includes(value)
    ? props.modelValue.filter((v) => v !== value)
    : [...props.modelValue, value]
  emit('update:modelValue', next)
}
</script>

<template>
  <div class="ui-filter-chips" role="group" :aria-label="label">
    <button
      class="ui-fchip"
      :class="{ 'is-selected': isAll }"
      type="button"
      :aria-pressed="isAll"
      @click="emit('update:modelValue', [])"
    >
      <UiIcon v-if="isAll" name="check" :size="20" />
      {{ allLabel }}
    </button>
    <button
      v-for="o in options"
      :key="o.value"
      class="ui-fchip"
      :class="{ 'is-selected': modelValue.includes(o.value) }"
      type="button"
      :aria-pressed="modelValue.includes(o.value)"
      @click="toggle(o.value)"
    >
      <UiIcon v-if="modelValue.includes(o.value)" name="check" :size="20" />
      <UiIcon v-else-if="o.icon" :name="o.icon" :size="20" />
      <span
        v-if="o.color"
        class="ui-fchip__dot"
        :style="{ background: o.color }"
        aria-hidden="true"
      />
      {{ o.label }}
    </button>
  </div>
</template>

<style scoped>
.ui-filter-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
  min-width: 0;
}
.ui-fchip {
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
.ui-fchip:hover {
  background: var(--ui-state-hover);
}
.ui-fchip.is-selected {
  border-color: transparent;
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.ui-fchip:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
.ui-fchip__dot {
  display: inline-block;
  width: 0.75rem;
  height: 0.75rem;
  border-radius: var(--ui-radius-full);
}
</style>
