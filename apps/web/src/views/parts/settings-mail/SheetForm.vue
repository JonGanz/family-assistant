<script setup lang="ts">
// A create/edit form in a UiSheet: the body is a form, Save and Cancel sit in the sheet actions.
import { useId } from 'vue'
import { UiButton, UiSheet } from '../../../ui'

defineProps<{
  modelValue: boolean
  title: string
  saveDisabled?: boolean
  saveLabel?: string
  saving?: boolean
}>()
defineEmits<{ 'update:modelValue': [v: boolean]; save: [] }>()
const formId = useId()
</script>

<template>
  <UiSheet
    :model-value="modelValue"
    :title="title"
    @update:model-value="$emit('update:modelValue', $event)"
  >
    <form :id="formId" class="sheet-form" autocomplete="off" @submit.prevent="$emit('save')">
      <slot />
    </form>
    <template #actions>
      <UiButton variant="text" @click="$emit('update:modelValue', false)">Cancel</UiButton>
      <UiButton type="submit" :form="formId" :disabled="saveDisabled" :loading="saving">{{
        saveLabel ?? 'Save'
      }}</UiButton>
    </template>
  </UiSheet>
</template>

<style scoped>
.sheet-form {
  display: grid;
  gap: var(--ui-space-3);
}
</style>
