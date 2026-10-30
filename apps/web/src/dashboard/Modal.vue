<script setup lang="ts">
import { S } from './strings'

defineProps<{ title: string }>()
const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <div class="backdrop" data-testid="detail-modal" @click.self="emit('close')">
    <section class="modal" role="dialog" aria-modal="true" :aria-label="title">
      <header>
        <h2>{{ title }}</h2>
        <button type="button" class="close" :aria-label="S.close" @click="emit('close')">✕</button>
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
  width: min(60rem, 100%);
  max-height: 100%;
  background: var(--color-bg);
  color: var(--color-text);
  border: 2px solid var(--color-border);
  font-size: var(--font-size-base);
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  background: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
}
h2 {
  margin: 0;
  font-size: var(--font-size-lg);
}
.close {
  min-width: var(--min-touch);
  min-height: var(--min-touch);
  border: 2px solid var(--color-border);
  background: var(--color-bg);
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.body {
  overflow-y: auto;
  padding: var(--space-4);
}
</style>
