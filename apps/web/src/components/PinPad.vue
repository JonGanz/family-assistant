<script setup lang="ts">
import { computed, ref } from 'vue'
import UiIcon from '../ui/UiIcon.vue'

const props = withDefaults(defineProps<{ minLength?: number; maxLength?: number }>(), {
  minLength: 4,
  maxLength: 6,
})
const emit = defineEmits<{ submit: [pin: string] }>()

const pin = ref('')
const canSubmit = computed(() => pin.value.length >= props.minLength)
const DIGITS = ['1', '2', '3', '4', '5', '6', '7', '8', '9']

function press(d: string) {
  if (pin.value.length < props.maxLength) pin.value += d
}
function back() {
  pin.value = pin.value.slice(0, -1)
}
function submit() {
  if (!canSubmit.value) return
  emit('submit', pin.value)
  pin.value = ''
}
defineExpose({ clear: () => (pin.value = '') })
</script>

<template>
  <div class="pinpad">
    <output
      class="display"
      data-testid="pin-display"
      :data-count="pin.length"
      :aria-label="`${pin.length} of ${maxLength} digits`"
    >
      <span v-for="i in maxLength" :key="i" class="dot" :class="{ on: i <= pin.length }"></span>
    </output>
    <div class="keys">
      <button v-for="d in DIGITS" :key="d" type="button" class="key" @click="press(d)">
        {{ d }}
      </button>
      <button
        type="button"
        class="key quiet"
        aria-label="Backspace"
        :disabled="!pin.length"
        @click="back"
      >
        <UiIcon name="backspace" :size="24" />
      </button>
      <button type="button" class="key" @click="press('0')">0</button>
      <button
        type="button"
        class="key ok"
        :disabled="!canSubmit"
        aria-label="Enter"
        @click="submit"
      >
        <UiIcon name="check" :size="24" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.pinpad {
  display: inline-grid;
  justify-items: center;
  gap: var(--space-4);
}
.display {
  display: flex;
  gap: var(--space-3);
  min-height: 1.25rem;
  align-items: center;
}
.dot {
  width: 0.9rem;
  height: 0.9rem;
  border-radius: 50%;
  border: 2px solid var(--color-muted);
  transition:
    background 0.12s,
    border-color 0.12s,
    transform 0.12s;
}
.dot.on {
  background: var(--color-accent);
  border-color: var(--color-accent);
  transform: scale(1.1);
}
.keys {
  display: grid;
  grid-template-columns: repeat(3, 4.5rem);
  gap: var(--space-3) var(--space-4);
}
.key {
  display: grid;
  place-items: center;
  width: 4.5rem;
  height: 4.5rem;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--color-layer-3, var(--color-surface));
  color: var(--color-text);
  font: inherit;
  font-size: 1.75rem;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  cursor: pointer;
  -webkit-tap-highlight-color: transparent;
  transition:
    background 0.1s,
    transform 0.1s;
}
.key:active:not(:disabled) {
  transform: scale(0.94);
  background: color-mix(
    in srgb,
    var(--color-accent) 25%,
    var(--color-layer-3, var(--color-surface))
  );
}
.key:focus-visible {
  outline: 3px solid var(--color-accent);
  outline-offset: 2px;
}
.quiet {
  background: transparent;
  color: var(--color-muted);
}
.ok {
  background: var(--color-accent);
  color: var(--color-on-accent);
}
.key:disabled {
  opacity: 0.35;
  cursor: default;
}
</style>
