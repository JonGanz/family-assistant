<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router'

defineProps<{
  title: string
  subtitle?: string
  /** Where the back link goes; omitted = no back link. */
  back?: RouteLocationRaw
  /** Small label above the title, e.g. "Settings". */
  eyebrow?: string
}>()
</script>

<template>
  <header class="page-header">
    <div class="page-header-text">
      <p v-if="eyebrow" class="page-eyebrow">{{ eyebrow }}</p>
      <h1>
        <RouterLink v-if="back" class="page-back" :to="back" aria-label="Back">‹</RouterLink>
        {{ title }}
      </h1>
      <p v-if="subtitle || $slots.default" class="page-subtitle">
        <slot>{{ subtitle }}</slot>
      </p>
    </div>
    <div v-if="$slots.actions" class="page-actions"><slot name="actions" /></div>
  </header>
</template>
