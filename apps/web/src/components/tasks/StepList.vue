<script setup lang="ts">
import { UiCheckbox, UiList } from '../../ui'
import type { StepModel } from '../../lib/tasks'

defineProps<{ steps: StepModel[]; disabled?: boolean }>()
const emit = defineEmits<{ tick: [stepId: string, done: boolean] }>()
</script>

<template>
  <UiList class="steps">
    <li v-for="s in steps" :key="s.id" :class="{ done: s.done }">
      <UiCheckbox
        :model-value="s.done"
        :disabled="disabled"
        :label="s.text"
        @update:model-value="emit('tick', s.id, $event)"
      />
    </li>
  </UiList>
</template>

<style scoped>
.steps {
  list-style: none;
  margin: 0;
  padding: 0;
}
.done :deep(span) {
  text-decoration: line-through;
  color: var(--ui-on-surface-variant);
}
</style>
