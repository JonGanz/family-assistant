<script setup lang="ts">
import UiBottomSheet from './UiBottomSheet.vue'
import UiDialog from './UiDialog.vue'
import { useBreakpoint } from './composables/useBreakpoint'

withDefaults(defineProps<{ modelValue: boolean; title: string; dismissible?: boolean }>(), {
  dismissible: true,
})
defineEmits<{ 'update:modelValue': [value: boolean] }>()
const bp = useBreakpoint()
</script>

<template>
  <component
    :is="bp === 'compact' ? UiBottomSheet : UiDialog"
    :model-value="modelValue"
    :title="title"
    :dismissible="dismissible"
    @update:model-value="$emit('update:modelValue', $event)"
  >
    <slot />
    <template v-if="$slots.actions" #actions><slot name="actions" /></template>
  </component>
</template>
