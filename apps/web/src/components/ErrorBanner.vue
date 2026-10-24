<script setup lang="ts">
defineProps<{ message?: string; conflict?: boolean }>()
defineEmits<{ reload: [] }>()
</script>

<template>
  <div class="error-banner" role="alert">
    <span v-if="conflict">This was changed elsewhere. Reload to see the latest version.</span>
    <span v-else>{{ message }}</span>
    <button v-if="conflict" type="button" @click="$emit('reload')">Reload</button>
  </div>
</template>

<style scoped>
.error-banner {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-md);
  color: var(--color-danger);
  background: var(--color-surface);
}
button {
  margin-left: auto;
}
</style>
