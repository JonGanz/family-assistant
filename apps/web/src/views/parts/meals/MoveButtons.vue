<script setup lang="ts">
import { UiIconButton } from '../../../ui'

defineProps<{ index: number; count: number; name: string }>()
defineEmits<{ move: [delta: number] }>()
</script>

<template>
  <span class="move">
    <UiIconButton
      icon="arrow_upward"
      :label="`Move ${name} up`"
      :disabled="index === 0"
      @click="$emit('move', -1)"
    />
    <UiIconButton
      icon="arrow_downward"
      :label="`Move ${name} down`"
      :disabled="index === count - 1"
      @click="$emit('move', 1)"
    />
  </span>
</template>

<style scoped>
.move {
  display: inline-flex;
}
</style>
