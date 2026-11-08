<script setup lang="ts">
import { computed } from 'vue'
import type { GroceryItemView } from '../../../api/food'
import { UiIconButton } from '../../../ui'
import { sourceHint } from './grouping'

const props = defineProps<{
  item: GroceryItemView
  kiosk?: boolean
  /** Show the edit button (adults, not on the kiosk). */
  editable?: boolean
  /** Show the delete button. */
  deletable?: boolean
  /** Edit and delete need the server, so they are off while offline. */
  offline?: boolean
}>()
const emit = defineEmits<{ toggle: [on: boolean]; edit: []; remove: [] }>()
const checked = computed(() => !!props.item.checked_at)
const hint = computed(() => sourceHint(props.item.sources))
</script>

<template>
  <li
    class="grow"
    :class="{ 'is-checked': checked, 'is-kiosk': kiosk }"
    :data-testid="`grocery-item-${item.id}`"
  >
    <label class="grow__check">
      <input
        type="checkbox"
        :checked="checked"
        :data-testid="`grocery-check-${item.id}`"
        @change="emit('toggle', ($event.target as HTMLInputElement).checked)"
      />
      <span class="grow__text">
        <span class="grow__title">
          {{ item.text }}<span v-if="item.display" class="grow__qty"> {{ item.display }}</span>
        </span>
        <span v-if="hint" class="grow__hint">{{ hint }}</span>
      </span>
    </label>
    <UiIconButton
      v-if="editable"
      icon="edit"
      :label="`Edit ${item.text}`"
      :disabled="offline"
      @click="emit('edit')"
    />
    <UiIconButton
      v-if="deletable"
      icon="delete"
      :label="`Delete ${item.text}`"
      :disabled="offline"
      @click="emit('remove')"
    />
  </li>
</template>

<style scoped>
.grow {
  display: flex;
  align-items: center;
  gap: var(--ui-space-1);
  min-height: var(--ui-row-height);
  border-bottom: 1px solid var(--ui-outline-variant);
}
.grow__check {
  flex: 1;
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
  min-height: var(--ui-touch);
  min-width: 0;
  cursor: pointer;
}
input[type='checkbox'] {
  flex: none;
  width: 24px;
  height: 24px;
  margin: 0 var(--ui-space-1);
  accent-color: var(--ui-primary);
}
.grow__text {
  display: grid;
  min-width: 0;
}
.grow__title {
  font: var(--ui-type-body-lg);
  color: var(--ui-on-surface);
  overflow-wrap: anywhere;
}
.grow__qty {
  color: var(--ui-on-surface-variant);
}
.grow__hint {
  font: var(--ui-type-body-sm);
  color: var(--ui-on-surface-variant);
}
.is-checked .grow__title {
  text-decoration: line-through;
  color: var(--ui-on-surface-variant);
}
.is-kiosk {
  min-height: 64px;
}
.is-kiosk input[type='checkbox'] {
  width: 32px;
  height: 32px;
}
.is-kiosk .grow__title {
  font: var(--ui-type-title-lg);
}
</style>
