<script setup lang="ts">
import { ref } from 'vue'
import { UiChip, UiTextField } from '../../../ui'

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
    <div v-if="modelValue.length" class="chips__row">
      <UiChip v-for="v in modelValue" :key="v" variant="input" @remove="remove(v)">{{ v }}</UiChip>
    </div>
    <UiTextField
      v-model="draft"
      :label="label"
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
  gap: var(--ui-space-2);
}
.chips__row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
</style>
