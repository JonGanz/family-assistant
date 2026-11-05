<script setup lang="ts">
import { UiButton } from '../../ui'

withDefaults(
  defineProps<{
    back?: boolean
    next?: string
    disabled?: boolean
    skip?: boolean
    busy?: boolean
  }>(),
  { back: true, next: 'Next' },
)
defineEmits<{ back: []; skip: [] }>()
</script>

<template>
  <div class="step-buttons">
    <UiButton v-if="back" variant="text" @click="$emit('back')">Back</UiButton>
    <span class="step-buttons__gap" />
    <UiButton v-if="skip" variant="text" @click="$emit('skip')">Skip for now</UiButton>
    <UiButton type="submit" :disabled="disabled" :loading="busy">{{ next }}</UiButton>
  </div>
</template>

<style scoped>
.step-buttons {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.step-buttons__gap {
  flex: 1;
}
</style>
