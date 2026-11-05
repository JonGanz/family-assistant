<script setup lang="ts">
import UiButton from './UiButton.vue'
import UiIcon from './UiIcon.vue'

withDefaults(defineProps<{ message?: string; retryLabel?: string; canRetry?: boolean }>(), {
  canRetry: true,
  message: 'Something went wrong.',
  retryLabel: 'Try again',
})
defineEmits<{ retry: [] }>()
</script>

<template>
  <div class="ui-error" role="alert">
    <UiIcon name="error" class="ui-error__icon" />
    <p class="ui-error__text">{{ message }}</p>
    <UiButton v-if="canRetry" variant="tonal" icon="refresh" @click="$emit('retry')">{{
      retryLabel
    }}</UiButton>
  </div>
</template>

<style scoped>
.ui-error {
  display: grid;
  justify-items: center;
  gap: var(--ui-space-2);
  padding: var(--ui-space-5) var(--ui-space-4);
  border-radius: var(--ui-radius-md);
  background: var(--ui-error-container);
  color: var(--ui-on-error-container);
  text-align: center;
}
.ui-error__icon {
  color: var(--ui-error);
}
.ui-error__text {
  margin: 0;
  font: var(--ui-type-body-lg);
}
</style>
