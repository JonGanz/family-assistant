<script setup lang="ts">
// A checkbox or switch that applies at once with Undo (D-096). One component per control so a
// list of them can each own a useInstantSave.
import { ref, watch } from 'vue'
import { UiCheckbox, UiSwitch, useInstantSave } from '../../../ui'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    label: string
    save: (v: boolean) => Promise<unknown>
    saved?: string
    as?: 'checkbox' | 'switch'
    disabled?: boolean
  }>(),
  { as: 'checkbox', saved: 'Saved' },
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
  <UiSwitch
    v-if="as === 'switch'"
    :model-value="value"
    :label="label"
    :disabled="disabled"
    @update:model-value="set"
  />
  <UiCheckbox
    v-else
    :model-value="value"
    :label="label"
    :disabled="disabled"
    @update:model-value="set"
  />
</template>
