<script setup lang="ts">
import { computed, ref } from 'vue'

const props = withDefaults(defineProps<{ minLength?: number; maxLength?: number }>(), {
  minLength: 4,
  maxLength: 6,
})
const emit = defineEmits<{ submit: [pin: string] }>()

const pin = ref('')
const masked = computed(() => '•'.repeat(pin.value.length).padEnd(props.maxLength, '○'))
const canSubmit = computed(() => pin.value.length >= props.minLength)

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
    <output class="display" data-testid="pin-display">{{ masked }}</output>
    <div class="keys">
      <button v-for="d in ['1', '2', '3', '4', '5', '6', '7', '8', '9']" :key="d" @click="press(d)">
        {{ d }}
      </button>
      <button aria-label="Backspace" @click="back">⌫</button>
      <button @click="press('0')">0</button>
      <button class="ok" :disabled="!canSubmit" aria-label="Enter" @click="submit">OK</button>
    </div>
  </div>
</template>

<style scoped>
.pinpad {
  display: inline-block;
}
.display {
  display: block;
  text-align: center;
  font-size: 2.5rem;
  letter-spacing: 0.5rem;
  margin-bottom: 1rem;
}
.keys {
  display: grid;
  grid-template-columns: repeat(3, 5rem);
  gap: 0.75rem;
}
button {
  height: 5rem;
  font-size: 2rem;
  border-radius: 0.75rem;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text);
}
.ok {
  background: var(--color-accent);
  color: var(--color-on-accent);
}
button:disabled {
  opacity: 0.4;
}
</style>
