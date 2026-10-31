<script setup lang="ts">
import { HUES, hueVar, type Hue } from '../lib/hues'

const props = defineProps<{
  modelValue: Hue
  name: string
  legend?: string
  round?: boolean
  hideLegend?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [hue: Hue] }>()
const label = (h: string) => h.charAt(0).toUpperCase() + h.slice(1)
</script>

<template>
  <fieldset class="hue-picker">
    <legend :class="{ 'sr-only': props.hideLegend }">{{ legend ?? 'Colour' }}</legend>
    <label v-for="h in HUES" :key="h" class="swatch" :title="label(h)">
      <input
        type="radio"
        :name="name"
        :value="h"
        :checked="props.modelValue === h"
        @change="emit('update:modelValue', h)"
      />
      <span class="dot" :style="{ background: hueVar(h) }" aria-hidden="true" />
      <span class="sr-only">{{ label(h) }}</span>
    </label>
  </fieldset>
</template>

<style scoped>
.hue-picker {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  border: 0;
  padding: 0;
  margin: 0;
}
.swatch {
  position: relative;
  display: inline-flex;
}
.swatch input {
  position: absolute;
  opacity: 0;
  inset: 0;
  margin: 0;
  cursor: pointer;
}
.dot {
  width: var(--swatch-size, 2rem);
  height: var(--swatch-size, 2rem);
  border-radius: 50%;
  border: 2px solid var(--color-border);
}
.swatch input:checked + .dot {
  border-color: var(--swatch-glow, var(--color-text));
  box-shadow:
    0 0 0 2px var(--color-bg) inset,
    0 0 10px 3px var(--swatch-glow, transparent);
}
.swatch input:focus-visible + .dot {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}
</style>
