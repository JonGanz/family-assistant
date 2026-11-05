<script setup lang="ts">
// The create action of a page: an extended FAB that sits above the Save bar offset.
import { UiFab } from '../../../ui'

defineProps<{ label: string }>()
defineEmits<{ click: [] }>()
</script>

<template>
  <div class="fab">
    <UiFab icon="add" :label="label" extended @click="$emit('click')" />
  </div>
</template>

<style scoped>
.fab {
  position: fixed;
  right: calc(var(--ui-space-4) + env(safe-area-inset-right));
  bottom: calc(var(--ui-bottom-offset, 0px) + var(--ui-space-4) + env(safe-area-inset-bottom));
  z-index: 15;
}
</style>
