<script setup lang="ts">
// "Today's meals" (D-117): today's plan entries in visible slots; hidden when there are none.
import { computed, onMounted, ref } from 'vue'
import { food, type MealPlanEntryView } from '../../api/food'
import { dayKey } from '../../lib/time'
import { useHouseholdStore } from '../../stores/household'
import { useMealsSettingsStore } from '../../stores/mealsSettings'
import { UiCard, UiList, UiSection } from '../../ui'

const household = useHouseholdStore()
const meals = useMealsSettingsStore()
const entries = ref<MealPlanEntryView[]>([])

const rows = computed(() =>
  meals.visibleSlots.flatMap((s) =>
    entries.value
      .filter((e) => e.slot_id === s.id)
      .sort((a, b) => a.ord - b.ord)
      .map((e) => ({ id: e.id, slot: s.name, title: e.title })),
  ),
)

onMounted(async () => {
  try {
    const today = dayKey(new Date(), household.tz)
    const [list] = await Promise.all([
      food.plan(today, today),
      meals.loaded ? Promise.resolve() : meals.load(),
    ])
    entries.value = list
  } catch {
    entries.value = []
  }
})
</script>

<template>
  <UiSection v-if="rows.length" heading="Today's meals" data-testid="today-meals">
    <UiCard variant="filled" to="/food/plan">
      <UiList aria-label="Today's meals">
        <li v-for="r in rows" :key="r.id" class="row">
          <span class="slot">{{ r.slot }}</span> {{ r.title }}
        </li>
      </UiList>
    </UiCard>
  </UiSection>
</template>

<style scoped>
.row {
  padding: var(--ui-space-1) var(--ui-space-3);
  min-height: var(--ui-touch);
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.slot {
  color: var(--ui-on-surface-variant);
}
</style>
