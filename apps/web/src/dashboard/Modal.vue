<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import UiIcon from '../ui/UiIcon.vue'
import { S } from './strings'

defineProps<{ title: string; fullscreen?: boolean }>()
const emit = defineEmits<{ close: [] }>()

/** A popup nobody touches for this long closes itself, so the dashboard returns to its view. */
const IDLE_CLOSE_MS = 60_000
let timer: ReturnType<typeof setTimeout> | undefined
function arm() {
  clearTimeout(timer)
  timer = setTimeout(() => emit('close'), IDLE_CLOSE_MS)
}
onMounted(arm)
onBeforeUnmount(() => clearTimeout(timer))
</script>

<template>
  <div
    class="backdrop"
    data-testid="detail-modal"
    @click.self="emit('close')"
    @pointerdown.passive="arm"
    @scroll.capture.passive="arm"
  >
    <section
      class="modal"
      :class="{ fullscreen }"
      role="dialog"
      aria-modal="true"
      :aria-label="title"
    >
      <header>
        <h2>{{ title }}</h2>
        <button type="button" class="close" :aria-label="S.close" @click="emit('close')">
          <UiIcon name="close" :size="24" />
        </button>
      </header>
      <div class="body"><slot /></div>
    </section>
  </div>
</template>

<style scoped>
.backdrop {
  position: absolute;
  inset: 0;
  z-index: 10;
  display: grid;
  place-items: center;
  padding: var(--space-5);
  background: rgb(0 0 0 / 55%);
}
.modal {
  display: flex;
  flex-direction: column;
  width: fit-content;
  min-width: min(20rem, 100%);
  max-width: min(60rem, 100%);
  max-height: 100%;
  overflow: hidden;
  background: var(--color-layer-2);
  color: var(--color-text);
  border-radius: var(--ui-radius-xl);
  box-shadow: 0 8px 32px rgb(0 0 0 / 35%);
  font-size: var(--font-size-base);
}
.modal.fullscreen {
  width: 100%;
  max-width: none;
  height: 100%;
  border-radius: var(--ui-radius-lg);
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-3) var(--space-2) var(--space-5);
}
h2 {
  margin: 0;
  font-size: var(--font-size-lg);
}
.close {
  display: grid;
  place-items: center;
  min-width: var(--min-touch);
  min-height: var(--min-touch);
  padding: 0;
  border: 0;
  border-radius: var(--ui-radius-full);
  background: var(--color-layer-3);
  color: inherit;
  cursor: pointer;
}
.body {
  overflow-y: auto;
  padding: var(--space-2) var(--space-5) var(--space-5);
}
</style>
