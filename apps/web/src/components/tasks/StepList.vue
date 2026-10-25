<script setup lang="ts">
import type { StepModel } from '../../lib/tasks'

defineProps<{ steps: StepModel[]; disabled?: boolean }>()
const emit = defineEmits<{ tick: [stepId: string, done: boolean] }>()
</script>

<template>
  <ul class="steps">
    <li v-for="s in steps" :key="s.id" :class="{ done: s.done }">
      <label>
        <input
          type="checkbox"
          :checked="s.done"
          :disabled="disabled"
          @change="emit('tick', s.id, ($event.target as HTMLInputElement).checked)"
        />
        <span class="text">{{ s.text }}</span>
      </label>
    </li>
  </ul>
</template>

<style scoped>
.steps {
  list-style: none;
  margin: 0;
  padding: 0;
}
label {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-height: var(--min-touch, 48px);
  cursor: pointer;
}
input {
  width: 1.4rem;
  height: 1.4rem;
  accent-color: var(--color-success);
}
.done .text {
  text-decoration: line-through;
  color: var(--color-muted);
}
</style>
