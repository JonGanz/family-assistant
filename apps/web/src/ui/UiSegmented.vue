<script setup lang="ts">
import { ref } from 'vue'

export interface SegmentOption {
  value: string
  label: string
}
const props = defineProps<{ modelValue: string; options: SegmentOption[]; label: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const buttons = ref<HTMLButtonElement[]>([])

function move(delta: number, from: number) {
  const n = props.options.length
  const i = (from + delta + n) % n
  emit('update:modelValue', props.options[i]!.value)
  buttons.value[i]?.focus()
}
function onKey(e: KeyboardEvent, i: number) {
  if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
    e.preventDefault()
    move(1, i)
  } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
    e.preventDefault()
    move(-1, i)
  }
}
</script>

<template>
  <div class="ui-seg" role="radiogroup" :aria-label="label">
    <button
      v-for="(o, i) in options"
      :key="o.value"
      ref="buttons"
      type="button"
      role="radio"
      class="ui-seg__btn"
      :class="{ 'is-on': o.value === modelValue }"
      :aria-checked="o.value === modelValue"
      :tabindex="o.value === modelValue ? 0 : -1"
      @click="emit('update:modelValue', o.value)"
      @keydown="onKey($event, i)"
    >
      {{ o.label }}
    </button>
  </div>
</template>

<style scoped>
.ui-seg {
  display: inline-flex;
  max-width: 100%;
  border: 1px solid var(--ui-outline);
  border-radius: var(--ui-radius-full);
  overflow: hidden;
}
.ui-seg__btn {
  flex: 1;
  min-width: 0;
  min-height: var(--ui-touch);
  padding: 0 var(--ui-space-4);
  border: 0;
  border-left: 1px solid var(--ui-outline);
  background: transparent;
  color: var(--ui-on-surface);
  font: var(--ui-type-label-lg);
  cursor: pointer;
}
.ui-seg__btn:first-child {
  border-left: 0;
}
.ui-seg__btn:hover {
  background: var(--ui-state-hover);
}
.ui-seg__btn.is-on {
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.ui-seg__btn:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: -3px;
}
</style>
