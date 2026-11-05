<script setup lang="ts">
import UiIcon from './UiIcon.vue'
import type { IconName } from './icons'

withDefaults(defineProps<{ title: string; text?: string; icon?: IconName }>(), { icon: 'inbox' })
</script>

<template>
  <div class="ui-empty">
    <UiIcon :name="icon" class="ui-empty__icon" />
    <p class="ui-empty__title">{{ title }}</p>
    <p v-if="text" class="ui-empty__text">{{ text }}</p>
    <div v-if="$slots.default" class="ui-empty__actions"><slot /></div>
  </div>
</template>

<style scoped>
.ui-empty {
  display: grid;
  justify-items: center;
  gap: var(--ui-space-2);
  padding: var(--ui-space-6) var(--ui-space-4);
  text-align: center;
  color: var(--ui-on-surface-variant);
}
.ui-empty__icon {
  width: 48px;
  height: 48px;
  color: var(--ui-outline);
}
.ui-empty__title {
  margin: 0;
  font: var(--ui-type-title-md);
  color: var(--ui-on-surface);
}
.ui-empty__text {
  margin: 0;
  max-width: 32ch;
  font: var(--ui-type-body-md);
}
.ui-empty__actions {
  margin-top: var(--ui-space-2);
}
</style>
