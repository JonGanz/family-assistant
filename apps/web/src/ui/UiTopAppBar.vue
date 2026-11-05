<script setup lang="ts">
import { inject } from 'vue'
import UiIconButton from './UiIconButton.vue'
import { useBreakpoint } from './composables/useBreakpoint'
import { navControlKey } from './nav'

defineProps<{ title: string; back?: boolean; backLabel?: string }>()
defineEmits<{ back: [] }>()
// Inside the app shell on a phone the bar carries the menu button (no bottom bar there).
const nav = inject(navControlKey, null)
const bp = useBreakpoint()
</script>

<template>
  <header class="ui-top">
    <UiIconButton v-if="back" icon="back" :label="backLabel ?? 'Back'" @click="$emit('back')" />
    <UiIconButton
      v-else-if="nav && bp === 'compact'"
      icon="menu"
      label="Open navigation"
      @click="nav.openNav()"
    />
    <h1 class="ui-top__title">{{ title }}</h1>
    <div class="ui-top__actions"><slot name="actions" /></div>
  </header>
</template>

<style scoped>
.ui-top {
  position: sticky;
  top: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  gap: var(--ui-space-1);
  min-height: 64px;
  padding: 0 var(--ui-space-2) 0 var(--ui-space-4);
  background: var(--ui-surface);
  color: var(--ui-on-surface);
  padding-top: env(safe-area-inset-top);
}
.ui-top > :first-child:not(h1) {
  margin-left: calc(-1 * var(--ui-space-2));
}
.ui-top__title {
  flex: 1;
  min-width: 0;
  margin: 0;
  overflow: hidden;
  font: var(--ui-type-title-lg);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ui-top__actions {
  flex: none;
  display: flex;
  align-items: center;
}
</style>
