<script setup lang="ts">
// "Update the shopping list?" after a plan change that touches unchecked list items (D-116).
import type { GroceryImpact } from '../../../api/food'
import { UiButton, UiDialog } from '../../../ui'

defineProps<{ items: GroceryImpact[]; busy?: boolean }>()
defineEmits<{ update: []; leave: [] }>()

const qty = (n: number | null | undefined, unit: string | null | undefined) =>
  n == null ? '' : `${Math.round(n * 100) / 100}${unit ? ` ${unit}` : ''}`

function describe(i: GroceryImpact): string {
  if (i.remove) return 'removed'
  const from = qty(i.from_qty, i.unit)
  const to = qty(i.to_qty, i.unit)
  return from && to ? `${from} to ${to}` : to || 'changed'
}
</script>

<template>
  <UiDialog :model-value="items.length > 0" title="Update the shopping list?" :dismissible="false">
    <div data-testid="impact-dialog">
      <p class="impact__lead">This change affects items still on the list:</p>
      <ul class="impact__list">
        <li v-for="i in items" :key="`${i.item_id}-${i.entry_id}`" data-testid="impact-item">
          <strong>{{ i.text }}</strong
          >: {{ describe(i) }}
        </li>
      </ul>
    </div>
    <template #actions>
      <UiButton variant="text" :disabled="busy" data-testid="impact-leave" @click="$emit('leave')">
        Leave the list
      </UiButton>
      <UiButton :loading="busy" data-testid="impact-update" @click="$emit('update')">
        Update the list
      </UiButton>
    </template>
  </UiDialog>
</template>

<style scoped>
.impact__lead {
  margin: 0 0 var(--ui-space-2);
}
.impact__list {
  display: grid;
  gap: var(--ui-space-1);
  margin: 0;
  padding-left: var(--ui-space-5);
}
</style>
