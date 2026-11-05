<script setup lang="ts">
import { ref, toRef, useId } from 'vue'
import { useFocusTrap } from './composables/useFocusTrap'

/** Shared body of UiDialog and UiBottomSheet (internal). */
const props = withDefaults(
  defineProps<{ open: boolean; title: string; kind: 'dialog' | 'sheet'; dismissible?: boolean }>(),
  { dismissible: true },
)
const emit = defineEmits<{ 'update:open': [value: boolean] }>()
const el = ref<HTMLElement | null>(null)
const titleId = useId()
function close() {
  if (props.dismissible) emit('update:open', false)
}
useFocusTrap(toRef(props, 'open'), el, close)
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="ui-overlay" :class="`is-${kind}`">
      <div class="ui-overlay__scrim" data-testid="scrim" @click="close" />
      <div
        ref="el"
        class="ui-overlay__panel"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        tabindex="-1"
      >
        <div v-if="kind === 'sheet'" class="ui-overlay__handle" aria-hidden="true" />
        <h2 :id="titleId" class="ui-overlay__title">{{ title }}</h2>
        <div class="ui-overlay__body"><slot /></div>
        <div v-if="$slots.actions" class="ui-overlay__actions"><slot name="actions" /></div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.ui-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ui-overlay.is-sheet {
  align-items: flex-end;
}
.ui-overlay__scrim {
  position: absolute;
  inset: 0;
  background: var(--ui-scrim);
}
.ui-overlay__panel {
  position: relative;
  display: flex;
  flex-direction: column;
  width: min(560px, calc(100% - 2 * var(--ui-space-4)));
  max-height: calc(100% - 2 * var(--ui-space-5));
  padding: var(--ui-space-5);
  border-radius: var(--ui-radius-xl);
  background: var(--ui-surface-container-high);
  color: var(--ui-on-surface);
  box-shadow: var(--ui-elev-3);
  animation: ui-pop var(--ui-motion-medium) var(--ui-ease);
}
.is-sheet .ui-overlay__panel {
  width: 100%;
  max-width: 640px;
  max-height: 85%;
  padding-bottom: calc(var(--ui-space-5) + env(safe-area-inset-bottom));
  border-radius: var(--ui-radius-xl) var(--ui-radius-xl) 0 0;
  background: var(--ui-surface-container-low);
  animation-name: ui-rise;
}
.ui-overlay__panel:focus {
  outline: none;
}
.ui-overlay__handle {
  align-self: center;
  width: 32px;
  height: 4px;
  margin: calc(-1 * var(--ui-space-3)) 0 var(--ui-space-3);
  border-radius: var(--ui-radius-full);
  background: var(--ui-outline);
}
.ui-overlay__title {
  margin: 0 0 var(--ui-space-4);
  font: var(--ui-type-headline-sm);
}
.ui-overlay__body {
  overflow: auto;
  font: var(--ui-type-body-md);
  color: var(--ui-on-surface-variant);
}
.ui-overlay__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--ui-space-2);
  margin-top: var(--ui-space-5);
}
@keyframes ui-pop {
  from {
    opacity: 0;
    transform: scale(0.94);
  }
}
@keyframes ui-rise {
  from {
    transform: translateY(40%);
    opacity: 0.5;
  }
}
</style>
