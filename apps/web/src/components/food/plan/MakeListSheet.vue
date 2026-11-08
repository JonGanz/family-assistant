<script setup lang="ts">
// Make grocery list: a date range (default the coming week) or meals picked in the grid (D-015, Frontend.md §3f).
import { computed, ref, watch } from 'vue'
import { useMealPlanStore } from '../../../stores/mealPlan'
import { useMealsSettingsStore } from '../../../stores/mealsSettings'
import { addDays } from '../../../lib/time'
import { UiButton, UiSegmented, UiSheet, UiTextField } from '../../../ui'

const props = defineProps<{
  modelValue: boolean
  defaultFrom: string
  saving?: boolean
  error?: string | null
}>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  range: [from: string, to: string]
  pick: []
}>()

const store = useMealPlanStore()
const settings = useMealsSettingsStore()
const mode = ref<'range' | 'pick'>('range')
const from = ref(props.defaultFrom)
const to = ref(addDays(props.defaultFrom, 6))

watch(
  () => props.modelValue,
  (open) => {
    if (!open) return
    mode.value = 'range'
    from.value = props.defaultFrom
    to.value = addDays(props.defaultFrom, 6)
  },
  { immediate: true },
)

const isDay = (v: string) => /^\d{4}-\d{2}-\d{2}$/.test(v)
const rangeOk = computed(() => isDay(from.value) && isDay(to.value) && to.value >= from.value)
/** Meals in visible slots inside the range, from the entries the grid already loaded. */
const count = computed(() => {
  const visible = new Set(settings.visibleSlots.map((s) => s.id))
  return store.entries.filter(
    (e) => e.date >= from.value && e.date <= to.value && visible.has(e.slot_id),
  ).length
})
</script>

<template>
  <UiSheet
    :model-value="modelValue"
    title="Make grocery list"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <div class="make" data-testid="make-list-sheet">
      <UiSegmented
        label="How to choose meals"
        :model-value="mode"
        :options="[
          { value: 'range', label: 'Date range' },
          { value: 'pick', label: 'Pick meals' },
        ]"
        @update:model-value="mode = $event as 'range' | 'pick'"
      />
      <template v-if="mode === 'range'">
        <div class="make__dates">
          <UiTextField v-model="from" type="date" label="From" data-testid="make-list-from" />
          <UiTextField
            v-model="to"
            type="date"
            label="To"
            :error="rangeOk ? undefined : 'Not before the start'"
            data-testid="make-list-to"
          />
        </div>
        <p class="make__summary" data-testid="make-list-summary">
          {{ count }} {{ count === 1 ? 'meal' : 'meals' }} in this range. Their ingredients are
          added to the list; staples are checked separately.
        </p>
      </template>
      <p v-else class="make__summary">Choose the meals in the grid, then make the list.</p>
      <p v-if="error" class="make__error" role="alert">{{ error }}</p>
    </div>
    <template #actions>
      <UiButton variant="text" @click="emit('update:modelValue', false)">Cancel</UiButton>
      <UiButton
        v-if="mode === 'range'"
        :disabled="!rangeOk || count === 0"
        :loading="saving"
        data-testid="make-list-confirm"
        @click="emit('range', from, to)"
      >
        Make list
      </UiButton>
      <UiButton v-else data-testid="make-list-pick" @click="emit('pick')">Choose meals</UiButton>
    </template>
  </UiSheet>
</template>

<style scoped>
.make {
  display: grid;
  gap: var(--ui-space-3);
}
.make__dates {
  display: grid;
  gap: var(--ui-space-3);
}
.make__summary {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.make__error {
  margin: 0;
  color: var(--ui-error);
}
@media (min-width: 600px) {
  .make__dates {
    grid-template-columns: 1fr 1fr;
  }
}
</style>
