<script setup lang="ts">
import UiButton from './UiButton.vue'
import { useSnackbar } from './composables/useSnackbar'

const { state, dismiss } = useSnackbar()
async function run() {
  const a = state.current?.action
  dismiss()
  await a?.run()
}
</script>

<template>
  <div class="ui-snackbar-host" role="status" aria-live="polite" aria-atomic="true">
    <div v-if="state.current" :key="state.current.id" class="ui-snackbar">
      <span class="ui-snackbar__text">{{ state.current.text }}</span>
      <UiButton
        v-if="state.current.action"
        variant="text"
        class="ui-snackbar__action"
        @click="run"
        >{{ state.current.action.label }}</UiButton
      >
    </div>
  </div>
</template>

<style scoped>
.ui-snackbar-host {
  position: fixed;
  left: 0;
  right: 0;
  bottom: calc(var(--ui-bottom-offset, 0px) + var(--ui-space-4) + env(safe-area-inset-bottom));
  z-index: 60;
  display: flex;
  justify-content: center;
  padding: 0 var(--ui-space-4);
  pointer-events: none;
}
.ui-snackbar {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  width: min(100%, 560px);
  min-height: 48px;
  padding: var(--ui-space-2) var(--ui-space-2) var(--ui-space-2) var(--ui-space-4);
  border-radius: var(--ui-radius-xs);
  background: var(--ui-inverse-surface);
  color: var(--ui-inverse-on-surface);
  box-shadow: var(--ui-elev-3);
  pointer-events: auto;
  animation: ui-snack var(--ui-motion-medium) var(--ui-ease);
}
.ui-snackbar__text {
  flex: 1;
  font: var(--ui-type-body-md);
}
.ui-snackbar__action {
  color: var(--ui-inverse-primary);
}
@keyframes ui-snack {
  from {
    opacity: 0;
    transform: translateY(16px);
  }
}
</style>
