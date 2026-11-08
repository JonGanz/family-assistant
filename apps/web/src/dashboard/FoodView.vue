<script setup lang="ts">
// The kiosk's Food view (D-117): the meal plan and the shopping list, read with the device token.
// Adding and checking items needs no PIN; edit and delete need the elevated adult (D-015, D-110).
import { computed, ref } from 'vue'
import { foodKiosk } from '../api/food'
import MealPlanGrid from '../components/food/plan/MealPlanGrid.vue'
import ShoppingList from '../components/food/grocery/ShoppingList.vue'
import { S } from './strings'
import { weekStartKey, type WeekStart } from './weekModel'

const props = defineProps<{
  /** `YYYY-MM-DD` in the household's zone. */
  today: string
  weekStart: WeekStart
  slots: { id: string; name: string; time?: string }[]
  /** The device is PIN-elevated. */
  elevated: boolean
}>()

type Tab = 'plan' | 'shopping'
const tab = ref<Tab>('plan')
const TABS: { id: Tab; label: string }[] = [
  { id: 'plan', label: S.foodPlan },
  { id: 'shopping', label: S.foodShopping },
]
/** Three week rows: last week, this week and next. */
const from = computed(() => weekStartKey(props.today, props.weekStart, -1))
</script>

<template>
  <div class="food" data-testid="food-view">
    <div class="tabs" role="tablist" :aria-label="S.foodTabs">
      <button
        v-for="t in TABS"
        :key="t.id"
        type="button"
        role="tab"
        :aria-selected="tab === t.id"
        :class="{ on: tab === t.id }"
        :data-testid="`food-tab-${t.id}`"
        @click="tab = t.id"
      >
        {{ t.label }}
      </button>
    </div>
    <div class="body" role="tabpanel">
      <MealPlanGrid
        v-if="tab === 'plan'"
        :api="foodKiosk"
        :slots="slots"
        :today="today"
        :from="from"
        :weeks="3"
        :editable="false"
        fill
      />
      <ShoppingList v-else :can-edit="elevated" kiosk :api="foodKiosk" />
    </div>
  </div>
</template>

<style scoped>
.food {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
  box-sizing: border-box;
}
.tabs {
  flex: none;
  display: flex;
  gap: var(--space-3);
}
.tabs button {
  flex: 1;
  min-height: 4rem;
  border: 2px solid var(--color-border);
  background: var(--color-surface);
  color: inherit;
  font: inherit;
  font-size: var(--font-size-lg);
  cursor: pointer;
}
.tabs button.on {
  background: var(--color-accent);
  border-color: var(--color-accent);
  color: var(--color-on-accent);
  font-weight: 600;
}
.body {
  flex: 1;
  min-height: 0;
  /* The plan fills this box and never scrolls; the list scrolls inside it. */
  overflow-y: auto;
  position: relative;
}
</style>
