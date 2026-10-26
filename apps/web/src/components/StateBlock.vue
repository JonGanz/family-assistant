<script setup lang="ts">
import ErrorBanner from './ErrorBanner.vue'

defineProps<{
  loading?: boolean
  error?: string | null
  /** True when the data loaded and is empty. */
  empty?: boolean
  emptyText?: string
  loadingText?: string
}>()
</script>

<template>
  <div class="state-block">
    <ErrorBanner v-if="error" :message="error" />
    <p v-if="loading" class="muted" role="status" aria-live="polite">
      {{ loadingText ?? 'Loading…' }}
    </p>
    <p v-else-if="empty && !error" class="muted">
      <slot>{{ emptyText ?? 'Nothing here yet.' }}</slot>
    </p>
  </div>
</template>
