<script setup lang="ts">
// A stand-alone single choice that applies at once with Undo (D-096).
import { ref, watch } from 'vue'
import { UiSelect, useInstantSave, type SelectOption } from '../../../ui'

const props = withDefaults(
  defineProps<{
    modelValue: string
    label: string
    options: SelectOption[]
    save: (v: string) => Promise<unknown>
    saved?: string
  }>(),
  { saved: 'Saved' },
)
const value = ref(props.modelValue)
watch(
  () => props.modelValue,
  (v) => (value.value = v),
)
const { set } = useInstantSave(
  value,
  async (v) => {
    if ((await props.save(v)) === false)
      throw new Error('Changed elsewhere. Reload to see the latest, then try again.')
  },
  { label: props.saved },
)
</script>

<template>
  <UiSelect :model-value="value" :label="label" :options="options" @update:model-value="set" />
</template>
