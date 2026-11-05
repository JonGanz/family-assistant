<script setup lang="ts">
import { UiButton, UiDialog } from '../../../ui'
import type { PendingConfirm } from './useConfirm'

defineProps<{ pending: PendingConfirm | null }>()
defineEmits<{ accept: []; cancel: [] }>()
</script>

<template>
  <UiDialog
    :model-value="!!pending"
    :title="pending?.title ?? ''"
    @update:model-value="(v: boolean) => v || $emit('cancel')"
  >
    <p>{{ pending?.text }}</p>
    <template #actions>
      <UiButton variant="text" @click="$emit('cancel')">Cancel</UiButton>
      <UiButton @click="$emit('accept')">{{ pending?.label }}</UiButton>
    </template>
  </UiDialog>
</template>
