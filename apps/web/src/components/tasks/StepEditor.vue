<script setup lang="ts">
import { ref } from 'vue'
import { UiButton, UiIconButton, UiTextField } from '../../ui'

const steps = defineModel<string[]>({ required: true })
defineProps<{ disabled?: boolean }>()

const newStep = ref('')
function move(i: number, d: number) {
  const j = i + d
  if (j < 0 || j >= steps.value.length) return
  const next = [...steps.value]
  ;[next[i], next[j]] = [next[j]!, next[i]!]
  steps.value = next
}
function add() {
  const t = newStep.value.trim()
  if (t) steps.value = [...steps.value, t]
  newStep.value = ''
}
function setText(i: number, text: string) {
  const next = [...steps.value]
  next[i] = text
  steps.value = next
}
function remove(i: number) {
  steps.value = steps.value.filter((_, k) => k !== i)
}
</script>

<template>
  <div class="step-editor">
    <ol v-if="steps.length" class="step-editor__list">
      <li v-for="(s, i) in steps" :key="i" class="step-editor__row">
        <div class="step-editor__field">
          <UiTextField
            :model-value="s"
            :label="`Step ${i + 1}`"
            :disabled="disabled"
            @update:model-value="setText(i, $event)"
          />
        </div>
        <UiIconButton
          icon="collapse"
          label="Move up"
          :disabled="disabled || i === 0"
          @click="move(i, -1)"
        />
        <UiIconButton
          icon="expand"
          label="Move down"
          :disabled="disabled || i === steps.length - 1"
          @click="move(i, 1)"
        />
        <UiIconButton icon="close" label="Remove step" :disabled="disabled" @click="remove(i)" />
      </li>
    </ol>
    <div class="step-editor__add">
      <div class="step-editor__field">
        <UiTextField
          v-model="newStep"
          label="Add a step"
          name="new_step"
          :disabled="disabled"
          @keydown.enter.prevent="add"
        />
      </div>
      <UiButton variant="tonal" icon="add" :disabled="disabled" @click="add">Add step</UiButton>
    </div>
  </div>
</template>

<style scoped>
.step-editor {
  display: grid;
  gap: var(--ui-space-3);
}
.step-editor__list {
  display: grid;
  gap: var(--ui-space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}
.step-editor__row,
.step-editor__add {
  display: flex;
  align-items: center;
  gap: var(--ui-space-1);
}
.step-editor__field {
  flex: 1;
  min-width: 0;
}
</style>
