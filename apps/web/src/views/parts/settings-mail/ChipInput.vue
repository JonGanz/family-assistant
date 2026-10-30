<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{ modelValue: string[]; label: string; placeholder?: string }>()
const emit = defineEmits<{ 'update:modelValue': [v: string[]] }>()
const draft = ref('')

function add() {
  const v = draft.value.trim()
  draft.value = ''
  if (v && !props.modelValue.includes(v)) emit('update:modelValue', [...props.modelValue, v])
}
function remove(v: string) {
  emit(
    'update:modelValue',
    props.modelValue.filter((x) => x !== v),
  )
}
</script>

<template>
  <div class="chips">
    <span class="label">{{ label }}</span>
    <ul>
      <li v-for="v in modelValue" :key="v">
        {{ v }}
        <button type="button" :aria-label="`Remove ${v}`" @click="remove(v)">✕</button>
      </li>
    </ul>
    <input
      v-model="draft"
      :aria-label="label"
      :placeholder="placeholder ?? 'Type and press Enter'"
      @keydown.enter.prevent="add"
      @blur="add"
      @keydown.,.prevent="add"
    />
  </div>
</template>

<style scoped>
.chips {
  display: grid;
  gap: var(--space-1);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}
li {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: 0 var(--space-2);
  border: 1px solid var(--color-border);
  border-radius: 0;
  font-size: var(--font-size-sm);
}
li button {
  border: 0;
  background: transparent;
  cursor: pointer;
  color: inherit;
}
</style>
