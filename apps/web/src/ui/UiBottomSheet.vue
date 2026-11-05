<script setup lang="ts">
import UiOverlay from './UiOverlay.vue'

withDefaults(defineProps<{ modelValue: boolean; title: string; dismissible?: boolean }>(), {
  dismissible: true,
})
defineEmits<{ 'update:modelValue': [value: boolean] }>()
</script>

<template>
  <UiOverlay
    kind="sheet"
    :open="modelValue"
    :title="title"
    :dismissible="dismissible"
    @update:open="$emit('update:modelValue', $event)"
  >
    <slot />
    <template v-if="$slots.actions" #actions><slot name="actions" /></template>
  </UiOverlay>
</template>
