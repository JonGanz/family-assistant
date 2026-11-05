<script setup lang="ts">
// Load error, 409 conflict and action error of a settings page, in design-system parts.
import { UiButton, UiCard, UiErrorState } from '../../../ui'

defineProps<{ loadError?: string | null; conflict?: boolean; error?: string }>()
defineEmits<{ reload: [] }>()
</script>

<template>
  <UiErrorState v-if="loadError" :message="loadError" @retry="$emit('reload')" />
  <UiCard v-if="conflict" variant="outlined" role="alert">
    <p>This page changed elsewhere. Reload to see the latest.</p>
    <template #actions
      ><UiButton variant="tonal" @click="$emit('reload')">Reload</UiButton></template
    >
  </UiCard>
  <UiCard v-if="error" variant="outlined" role="alert">
    <p>{{ error }}</p>
  </UiCard>
</template>
