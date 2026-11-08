<script setup lang="ts">
// Add or edit one plan entry (D-096: several fields, one Save). In edit mode the date and slot are the "Move to…"
// picker: another date or slot appends the entry there (D-120).
import { computed, ref, watch } from 'vue'
import type { FoodApi, MealPlanEntryView } from '../../../api/food'
import { UiButton, UiSelect, UiSheet, UiTextField } from '../../../ui'
import ContentPicker from './ContentPicker.vue'
import { emptyPick, pickFrom, pickValid, type ItemPick, type PlanSlot } from './planModel'

const props = defineProps<{
  modelValue: boolean
  slots: PlanSlot[]
  date: string
  slotId: string
  /** The entry being edited; null adds. */
  entry: MealPlanEntryView | null
  /** A recipe to start with (`?add=<recipe id>`). */
  recipe?: { id: string; title: string } | null
  minDate: string
  maxDate?: string
  api?: FoodApi
  saving?: boolean
  error?: string | null
}>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  save: [value: { date: string; slot_id: string; pick: ItemPick }]
  delete: []
}>()

const pick = ref<ItemPick>(emptyPick())
const date = ref(props.date)
const slotId = ref(props.slotId)

watch(
  () => props.modelValue,
  (open) => {
    if (!open) return
    date.value = props.date
    slotId.value = props.slotId
    pick.value = props.entry
      ? pickFrom(props.entry)
      : props.recipe
        ? { ...emptyPick(), recipe_id: props.recipe.id, recipe_title: props.recipe.title }
        : emptyPick()
  },
  { immediate: true },
)

const dateOk = computed(
  () =>
    /^\d{4}-\d{2}-\d{2}$/.test(date.value) &&
    date.value >= props.minDate &&
    (!props.maxDate || date.value <= props.maxDate),
)
const valid = computed(() => pickValid(pick.value) && dateOk.value && !!slotId.value)
const moved = computed(
  () => !!props.entry && (date.value !== props.entry.date || slotId.value !== props.entry.slot_id),
)
</script>

<template>
  <UiSheet
    :model-value="modelValue"
    :title="entry ? 'Edit entry' : 'Add to the plan'"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <form
      class="entry"
      data-testid="entry-sheet"
      @submit.prevent="valid && emit('save', { date, slot_id: slotId, pick })"
    >
      <ContentPicker v-model="pick" :api="api" />
      <p class="entry__head">{{ entry ? 'Move to…' : 'When' }}</p>
      <div class="entry__when">
        <UiTextField
          v-model="date"
          type="date"
          label="Date"
          :error="dateOk ? undefined : 'Pick a day inside the plan'"
          data-testid="entry-date"
        />
        <UiSelect
          v-model="slotId"
          label="Slot"
          :options="slots.map((s) => ({ value: s.id, label: s.name }))"
          data-testid="entry-slot"
        />
      </div>
      <p v-if="moved" class="entry__note">It is added after the entries already there.</p>
      <p v-if="error" class="entry__error" role="alert">{{ error }}</p>
    </form>
    <template #actions>
      <UiButton v-if="entry" variant="text" data-testid="entry-delete" @click="emit('delete')">
        Delete
      </UiButton>
      <UiButton variant="text" @click="emit('update:modelValue', false)">Cancel</UiButton>
      <UiButton
        :disabled="!valid"
        :loading="saving"
        data-testid="entry-save"
        @click="emit('save', { date, slot_id: slotId, pick })"
      >
        Save
      </UiButton>
    </template>
  </UiSheet>
</template>

<style scoped>
.entry {
  display: grid;
  gap: var(--ui-space-3);
}
.entry__head {
  margin: 0;
  font: var(--ui-type-title-sm);
}
.entry__when {
  display: grid;
  gap: var(--ui-space-3);
}
.entry__note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.entry__error {
  margin: 0;
  color: var(--ui-error);
  font: var(--ui-type-body-md);
}
@media (min-width: 600px) {
  .entry__when {
    grid-template-columns: 1fr 1fr;
  }
}
</style>
