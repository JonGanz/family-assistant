<script setup lang="ts">
// The meal plan grid (Frontend.md §3f): week rows of seven day columns, a day list on compact windows. The app's
// Meal plan view and the kiosk's Food view both use it (the kiosk passes `api`, `fill` and `slots`).
import { computed, ref, watch } from 'vue'
import { food, type FoodApi, type MealPlanEntryView } from '../../../api/food'
import { weekLabel } from '../../../dashboard/weekModel'
import { addDays, dayKey, formatDayHeading } from '../../../lib/time'
import { useHouseholdStore } from '../../../stores/household'
import { useMealPlanStore } from '../../../stores/mealPlan'
import { useMealsSettingsStore } from '../../../stores/mealsSettings'
import { UiIcon, useBreakpoint } from '../../../ui'
import { cellTestId, dayLabel, entryTitle, planSlots, weekRows, type PlanSlot } from './planModel'

const props = withDefaults(
  defineProps<{
    /** First day of the first week row (`YYYY-MM-DD`). */
    from: string
    weeks: number
    editable: boolean
    /** Rows share the height and nothing scrolls (the kiosk). */
    fill?: boolean
    /** The API to read with; the kiosk passes its device-token one. */
    api?: FoodApi
    /** Visible slots; the settings store's by default. */
    slots?: PlanSlot[]
    /** The household's today; computed from its time zone by default. */
    today?: string
    /** Last day the plan may be written (today plus the horizon). */
    maxDate?: string
    /** Selection mode (Make grocery list): chips toggle instead of opening. */
    selecting?: boolean
    selected?: string[]
  }>(),
  {
    fill: false,
    api: undefined,
    slots: undefined,
    today: undefined,
    maxDate: undefined,
    selecting: false,
    selected: () => [],
  },
)
const emit = defineEmits<{
  add: [date: string, slotId: string]
  select: [entry: MealPlanEntryView, editable: boolean]
  toggle: [entry: MealPlanEntryView]
  dropSlot: [entryId: string, date: string, slotId: string]
  swap: [a: string, b: string]
}>()

const store = useMealPlanStore()
const household = useHouseholdStore()
const settings = useMealsSettingsStore()
const bp = useBreakpoint()
const source = computed(() => props.api ?? food)

const todayKey = computed(() => props.today ?? dayKey(new Date(), household.tz))
const shown = computed<PlanSlot[]>(() => props.slots ?? planSlots(settings.visibleSlots))
const rows = computed(() => weekRows(props.from, props.weeks))
const compact = computed(() => bp.value === 'compact' && !props.fill)
const thumbs = computed(() => bp.value === 'expanded' && !props.fill)

watch(
  [() => props.from, () => props.weeks, source],
  ([from, weeks, api]) => {
    if (props.api) store.useApi(api)
    void store.load(from, addDays(from, weeks * 7 - 1))
  },
  { immediate: true },
)

const writable = (date: string) =>
  props.editable && date >= todayKey.value && (!props.maxDate || date <= props.maxDate)

// Drag and drop only where a pointer can drag; "Move to…" in the edit sheet covers touch (D-120).
const canDrag = computed(
  () =>
    props.editable &&
    !props.selecting &&
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(pointer: fine)').matches,
)
const dragging = ref<string | null>(null)
function onDragStart(e: DragEvent, entry: MealPlanEntryView) {
  dragging.value = entry.id
  e.dataTransfer?.setData('text/plain', entry.id)
  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
}
function dragId(e: DragEvent): string | null {
  return dragging.value ?? e.dataTransfer?.getData('text/plain') ?? null
}
function onDropSlot(e: DragEvent, date: string, slotId: string) {
  const id = dragId(e)
  dragging.value = null
  if (id && writable(date)) emit('dropSlot', id, date, slotId)
}
function onDropEntry(e: DragEvent, target: MealPlanEntryView) {
  const id = dragId(e)
  dragging.value = null
  if (id && id !== target.id && writable(target.date)) emit('swap', id, target.id)
}

const monthStart = (d: string) => d.endsWith('-01')
const headLabel = (d: string, first: boolean) =>
  compact.value
    ? formatDayHeading(d, household.tz, todayKey.value)
    : dayLabel(d, first || monthStart(d))
const hidden = ref(new Set<string>())
const isEmptyDay = (d: string) => shown.value.every((s) => store.cell(d, s.id).length === 0)
</script>

<template>
  <div
    class="plan-grid"
    :class="{ 'is-fill': fill, 'is-compact': compact }"
    data-testid="plan-grid"
  >
    <section v-for="(row, w) in rows" :key="row[0]" class="week" :data-week="row[0]">
      <h2 class="week__head">
        {{ weekLabel(row[0]!) }}
        <span v-if="row.includes(todayKey)" class="week__now">This week</span>
      </h2>
      <div class="week__days">
        <div
          v-for="(d, i) in row"
          :key="d"
          class="day"
          :class="{ 'is-today': d === todayKey, 'is-past': d < todayKey }"
          :data-day="d"
          :aria-current="d === todayKey ? 'date' : undefined"
        >
          <h3 class="day__head">{{ headLabel(d, w === 0 && i === 0) }}</h3>
          <div
            v-for="s in shown"
            :key="s.id"
            class="slot"
            :class="{ 'is-over': dragging && writable(d) }"
            :data-testid="cellTestId(d, s.name)"
            @dragover="writable(d) && dragging && $event.preventDefault()"
            @drop.prevent="onDropSlot($event, d, s.id)"
          >
            <span v-if="shown.length > 1" class="slot__name">{{ s.name }}</span>
            <ul class="slot__list">
              <li v-for="e in store.cell(d, s.id)" :key="e.id">
                <component
                  :is="fill ? 'span' : 'button'"
                  class="chip"
                  :class="{ 'is-selected': selected.includes(e.id) }"
                  :type="fill ? undefined : 'button'"
                  :draggable="canDrag && writable(d)"
                  :aria-pressed="selecting ? selected.includes(e.id) : undefined"
                  @dragstart="onDragStart($event, e)"
                  @dragend="dragging = null"
                  @dragover="writable(d) && dragging && $event.preventDefault()"
                  @drop.prevent.stop="onDropEntry($event, e)"
                  @click="selecting ? emit('toggle', e) : emit('select', e, writable(d))"
                >
                  <img
                    v-if="thumbs && e.recipe_id && !hidden.has(e.id)"
                    class="chip__img"
                    :src="source.recipeImageUrl(e.recipe_id)"
                    alt=""
                    loading="lazy"
                    @error="hidden.add(e.id)"
                  />
                  <span class="chip__title">{{ entryTitle(e) }}</span>
                </component>
              </li>
            </ul>
            <button
              v-if="writable(d) && !selecting"
              type="button"
              class="slot__add"
              :aria-label="`Add to ${s.name} on ${dayLabel(d, true)}`"
              :data-testid="`plan-slot-add-${d}-${s.name.toLowerCase()}`"
              @click="emit('add', d, s.id)"
            >
              <UiIcon name="add" :size="20" />
            </button>
          </div>
          <p v-if="compact && isEmptyDay(d) && !writable(d)" class="day__none">Nothing planned</p>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.plan-grid {
  display: grid;
  gap: var(--ui-space-4);
}
.week {
  display: grid;
  gap: var(--ui-space-2);
  min-width: 0;
}
.week__head {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  margin: 0;
  font: var(--ui-type-title-sm);
  color: var(--ui-on-surface-variant);
}
.week__now {
  padding: 0 var(--ui-space-2);
  border-radius: var(--ui-radius-full);
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
  font: var(--ui-type-label-sm);
}
.week__days {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: var(--ui-space-2);
}
.day {
  display: grid;
  align-content: start;
  gap: var(--ui-space-1);
  min-width: 0;
  padding: var(--ui-space-2);
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-sm);
  background: var(--ui-surface-container-low);
}
.day.is-past {
  background: var(--ui-surface);
  color: var(--ui-on-surface-variant);
}
/* Today is drawn inverted, like the calendar's marker. */
.day.is-today {
  border-color: var(--ui-inverse-surface);
  background: var(--ui-inverse-surface);
  color: var(--ui-inverse-on-surface);
}
.day__head {
  margin: 0;
  font: var(--ui-type-label-lg);
}
.day__none {
  margin: 0;
  font: var(--ui-type-body-sm);
  opacity: 0.7;
}
.slot {
  display: grid;
  gap: var(--ui-space-1);
  min-width: 0;
  border-radius: var(--ui-radius-xs);
}
.slot.is-over {
  outline: 2px dashed var(--ui-primary);
  outline-offset: 1px;
}
.slot__name {
  font: var(--ui-type-label-sm);
  opacity: 0.75;
}
.slot__list {
  display: grid;
  gap: var(--ui-space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}
.chip {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  width: 100%;
  min-height: var(--ui-touch);
  padding: var(--ui-space-1) var(--ui-space-2);
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-sm);
  background: var(--ui-surface-container-high);
  color: var(--ui-on-surface);
  font: var(--ui-type-body-md);
  text-align: left;
  cursor: pointer;
}
.chip[draggable='true'] {
  cursor: grab;
}
.chip:hover {
  background: var(--ui-surface-container-highest);
}
.chip:focus-visible,
.slot__add:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 1px;
}
.chip.is-selected {
  border-color: var(--ui-primary);
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.chip__img {
  flex: none;
  width: 32px;
  height: 32px;
  border-radius: var(--ui-radius-xs);
  object-fit: cover;
}
.chip__title {
  min-width: 0;
  overflow-wrap: anywhere;
}
.slot__add {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  min-height: var(--ui-touch);
  border: 1px dashed var(--ui-outline);
  border-radius: var(--ui-radius-sm);
  background: transparent;
  color: inherit;
  cursor: pointer;
}
.slot__add:hover {
  background: var(--ui-state-hover);
}

@media (min-width: 600px) {
  .week__days {
    grid-template-columns: repeat(7, minmax(0, 1fr));
  }
  .day {
    min-height: 7rem;
  }
}
/* The kiosk: rows share the height and nothing scrolls. */
.is-fill {
  height: 100%;
  grid-template-rows: repeat(v-bind('weeks'), minmax(0, 1fr));
  gap: var(--ui-space-2);
  overflow: hidden;
}
.is-fill .week {
  grid-template-rows: auto minmax(0, 1fr);
  min-height: 0;
}
.is-fill .week__days {
  grid-template-columns: repeat(7, minmax(0, 1fr));
  min-height: 0;
}
.is-fill .day {
  min-height: 0;
  overflow: hidden;
}
.is-fill .chip {
  min-height: 0;
}
</style>
