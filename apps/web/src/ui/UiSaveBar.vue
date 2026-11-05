<script setup lang="ts">
import UiButton from './UiButton.vue'

defineProps<{ dirty: boolean; saving?: boolean; error?: string | null }>()
defineEmits<{ save: []; discard: [] }>()
</script>

<template>
  <Transition name="ui-savebar">
    <div v-if="dirty || error" class="ui-savebar" role="region" aria-label="Unsaved changes">
      <span
        class="ui-savebar__text"
        :class="{ 'is-error': error }"
        :role="error ? 'alert' : undefined"
        >{{ error || 'Unsaved changes' }}</span
      >
      <UiButton variant="text" :disabled="saving" @click="$emit('discard')">Discard</UiButton>
      <UiButton :loading="saving" @click="$emit('save')">Save</UiButton>
    </div>
  </Transition>
</template>

<style scoped>
.ui-savebar {
  position: sticky;
  bottom: calc(var(--ui-bottom-offset, 0px) + var(--ui-space-2));
  z-index: 10;
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  padding: var(--ui-space-2) var(--ui-space-3) var(--ui-space-2) var(--ui-space-4);
  border-radius: var(--ui-radius-lg);
  background: var(--ui-surface-container-highest);
  color: var(--ui-on-surface);
  box-shadow: var(--ui-elev-2);
}
.ui-savebar__text {
  flex: 1;
  font: var(--ui-type-body-md);
}
.ui-savebar__text.is-error {
  color: var(--ui-error);
}
@media (min-width: 600px) {
  .ui-savebar {
    top: var(--ui-space-2);
    bottom: auto;
  }
}
.ui-savebar-enter-active,
.ui-savebar-leave-active {
  transition:
    opacity var(--ui-motion-medium) var(--ui-ease),
    transform var(--ui-motion-medium) var(--ui-ease);
}
.ui-savebar-enter-from,
.ui-savebar-leave-to {
  opacity: 0;
  transform: translateY(12px);
}
</style>
