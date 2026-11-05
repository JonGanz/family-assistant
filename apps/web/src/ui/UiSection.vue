<script setup lang="ts">
defineProps<{ heading: string; supporting?: string }>()
</script>

<template>
  <section class="ui-section">
    <header>
      <h2>{{ heading }}</h2>
      <p v-if="supporting">{{ supporting }}</p>
    </header>
    <slot />
  </section>
</template>

<style scoped>
.ui-section {
  display: grid;
  gap: var(--ui-space-3);
}
h2 {
  margin: 0;
  font: var(--ui-type-title-md);
  color: var(--ui-on-surface);
}
p {
  margin: 0;
  font: var(--ui-type-body-md);
  color: var(--ui-on-surface-variant);
}
</style>
