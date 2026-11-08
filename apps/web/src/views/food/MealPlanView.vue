<script setup lang="ts">
// Food → Meal plan (Frontend.md §3f): the grid from the previous week to the horizon, with the adult actions:
// add/edit/move/swap/delete, apply a template, make a grocery list, save as template, and the list-impact dialog.
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { food, type MealPlanEntryView } from '../../api/food'
import { isConflict } from '../../api/http'
import ApplyTemplateSheet from '../../components/food/plan/ApplyTemplateSheet.vue'
import EntrySheet from '../../components/food/plan/EntrySheet.vue'
import FoodTabs from '../../components/food/FoodTabs.vue'
import ImpactDialog from '../../components/food/plan/ImpactDialog.vue'
import MakeListSheet from '../../components/food/plan/MakeListSheet.vue'
import MealPlanGrid from '../../components/food/plan/MealPlanGrid.vue'
import SaveAsTemplateSheet from '../../components/food/plan/SaveAsTemplateSheet.vue'
import { pickPayload, planSlots, type ItemPick } from '../../components/food/plan/planModel'
import { weekStartKey, type WeekStart } from '../../dashboard/weekModel'
import { addDays, dayKey } from '../../lib/time'
import { useHouseholdStore } from '../../stores/household'
import { useMealPlanStore } from '../../stores/mealPlan'
import { useMealsSettingsStore } from '../../stores/mealsSettings'
import { useSessionStore } from '../../stores/session'
import { UiButton, UiErrorState, UiPage, UiSkeleton, UiTopAppBar, useSnackbar } from '../../ui'
import { describeError } from '../shared'

const session = useSessionStore()
const household = useHouseholdStore()
const settings = useMealsSettingsStore()
const store = useMealPlanStore()
const snackbar = useSnackbar()
const route = useRoute()
const router = useRouter()

const isAdult = computed(() => session.isAdult)
const today = computed(() => dayKey(new Date(), household.tz))
const horizon = computed(() => settings.settings?.meal_plan_horizon_weeks ?? 2)
const weekStart = computed<WeekStart>(() => (household.view?.week_start as WeekStart) ?? 'monday')
/** Past weeks shown beyond the previous one; "Show earlier weeks" adds four. */
const earlier = ref(0)
const from = computed(() => weekStartKey(today.value, weekStart.value, -1 - earlier.value))
const weeks = computed(() => earlier.value + 2 + horizon.value)
const maxDate = computed(() => addDays(today.value, horizon.value * 7))
const slots = computed(() => planSlots(settings.visibleSlots))
const ready = computed(() => household.loaded && settings.loaded)

onMounted(async () => {
  await Promise.all([
    household.loaded ? null : household.load(),
    settings.loaded ? null : settings.load(),
  ])
  await openFromQuery()
})

async function act<T>(fn: () => Promise<T>): Promise<T | undefined> {
  try {
    return await fn()
  } catch (e) {
    if (isConflict(e)) {
      void store.load(from.value, addDays(from.value, weeks.value * 7 - 1))
      snackbar.show({ text: 'That entry changed elsewhere. The plan was refreshed; try again.' })
    } else snackbar.show({ text: describeError(e) })
    return undefined
  }
}
const undoable = (text: string, changeId: string | null | undefined) =>
  snackbar.show({
    text,
    ...(changeId
      ? {
          action: {
            label: 'Undo',
            run: () => act(() => store.undo(changeId)).then(() => undefined),
          },
        }
      : {}),
  })

// --- Add / edit -----------------------------------------------------------------------------
const sheetOpen = ref(false)
const sheetEntry = ref<MealPlanEntryView | null>(null)
const sheetDate = ref('')
const sheetSlot = ref('')
const sheetRecipe = ref<{ id: string; title: string } | null>(null)
const sheetBusy = ref(false)
const sheetError = ref<string | null>(null)

function openAdd(
  date: string,
  slotId: string,
  recipe: { id: string; title: string } | null = null,
) {
  sheetEntry.value = null
  sheetDate.value = date
  sheetSlot.value = slotId
  sheetRecipe.value = recipe
  sheetError.value = null
  sheetOpen.value = true
}
const addToday = () => openAdd(today.value, slots.value[0]?.id ?? '')

function onSelect(e: MealPlanEntryView, editable: boolean) {
  if (editable && isAdult.value) {
    sheetEntry.value = e
    sheetDate.value = e.date
    sheetSlot.value = e.slot_id
    sheetRecipe.value = null
    sheetError.value = null
    sheetOpen.value = true
  } else if (e.recipe_id) void router.push(`/food/recipes/${e.recipe_id}`)
}

async function saveEntry(v: { date: string; slot_id: string; pick: ItemPick }) {
  sheetBusy.value = true
  sheetError.value = null
  try {
    const content = pickPayload(v.pick)
    const e = sheetEntry.value
    if (!e) {
      await store.add({ date: v.date, slot_id: v.slot_id, ...content })
      snackbar.show({ text: 'Added to the plan' })
    } else {
      const patch: Record<string, unknown> = {}
      if (v.date !== e.date) patch.date = v.date
      if (v.slot_id !== e.slot_id) patch.slot_id = v.slot_id
      const sameContent =
        content.recipe_id !== undefined
          ? content.recipe_id === e.recipe_id
          : !e.recipe_id && content.free_text === e.free_text
      if (!sameContent) {
        if (content.recipe_id) patch.recipe_id = content.recipe_id
        else patch.free_text = content.free_text
      }
      if (content.servings !== undefined && content.servings !== e.servings)
        patch.servings = content.servings
      if (Object.keys(patch).length === 0) {
        sheetOpen.value = false
        return
      }
      const res = await store.update(e, patch)
      undoable('Entry saved', res.change_id)
    }
    sheetOpen.value = false
  } catch (err) {
    sheetError.value = isConflict(err)
      ? 'This entry changed elsewhere. Close and open it again.'
      : describeError(err)
  } finally {
    sheetBusy.value = false
  }
}

async function deleteEntry() {
  const e = sheetEntry.value
  if (!e) return
  sheetBusy.value = true
  try {
    await store.remove(e)
    sheetOpen.value = false
    // The server has no undo for a delete: put the same content back at the end of its slot.
    snackbar.show({
      text: 'Entry deleted',
      action: {
        label: 'Undo',
        run: () =>
          act(() =>
            store.add({
              date: e.date,
              slot_id: e.slot_id,
              ...(e.recipe_id ? { recipe_id: e.recipe_id } : { free_text: e.free_text }),
              servings: e.servings,
            }),
          ).then(() => undefined),
      },
    })
  } catch (err) {
    sheetError.value = describeError(err)
  } finally {
    sheetBusy.value = false
  }
}

async function dropSlot(id: string, date: string, slotId: string) {
  const e = store.entries.find((x) => x.id === id)
  if (!e || (e.date === date && e.slot_id === slotId)) return
  const res = await act(() => store.update(e, { date, slot_id: slotId }))
  if (res) undoable('Entry moved', res.change_id)
}
async function swapEntries(a: string, b: string) {
  const res = await act(() => store.swap({ a, b }))
  if (res) undoable('Entries swapped', res.change_id)
}

// `?add=<recipe id>` from a recipe's "Add to plan".
async function openFromQuery() {
  const id = route.query.add
  if (typeof id !== 'string' || !isAdult.value) return
  const r = await act(() => food.recipe(id))
  void router.replace({ query: {} })
  if (r) openAdd(today.value, slots.value[0]?.id ?? '', { id: r.id, title: r.title })
}

// --- List impact (D-116) ---------------------------------------------------------------------
const impactBusy = ref(false)
async function updateList() {
  impactBusy.value = true
  const res = await act(() => store.reconcile())
  impactBusy.value = false
  if (res) undoable('Shopping list updated', res.change_id)
}

// --- Apply template, make list, save as template -----------------------------------------------
const applyOpen = ref(false)
const applyBusy = ref(false)
const applyError = ref<string | null>(null)
const nextWeek = computed(() => weekStartKey(today.value, weekStart.value, 1))
async function applyTemplate(
  id: string,
  body: { start: string; add_slots: { date: string; slot_id: string }[] },
) {
  applyBusy.value = true
  applyError.value = null
  try {
    const res = await store.apply(id, body)
    applyOpen.value = false
    undoable('Template applied', res.change_id)
  } catch (e) {
    applyError.value = describeError(e)
  } finally {
    applyBusy.value = false
  }
}

const listOpen = ref(false)
const listBusy = ref(false)
const listError = ref<string | null>(null)
const selecting = ref(false)
const selected = ref<string[]>([])
async function makeList(req: { from?: string; to?: string; entry_ids?: string[] }) {
  listBusy.value = true
  listError.value = null
  try {
    const r = await store.generate(req)
    listOpen.value = false
    selecting.value = false
    selected.value = []
    undoable(
      `Added ${r.added}, merged ${r.merged}${r.staples ? `, ${r.staples} staples to check` : ''}`,
      r.change_id,
    )
  } catch (e) {
    listError.value = describeError(e)
  } finally {
    listBusy.value = false
  }
}
function toggleSelected(e: MealPlanEntryView) {
  selected.value = selected.value.includes(e.id)
    ? selected.value.filter((x) => x !== e.id)
    : [...selected.value, e.id]
}
function startPicking() {
  listOpen.value = false
  selected.value = []
  selecting.value = true
}

const templateOpen = ref(false)
const templateBusy = ref(false)
const templateError = ref<string | null>(null)
const thisWeek = computed(() => weekStartKey(today.value, weekStart.value, 0))
async function saveTemplate(body: { name: string; from: string; to: string; slot_ids: string[] }) {
  templateBusy.value = true
  templateError.value = null
  try {
    const t = await store.fromPlan(body)
    templateOpen.value = false
    await router.push(`/food/templates/${t.id}`)
  } catch (e) {
    templateError.value = describeError(e)
  } finally {
    templateBusy.value = false
  }
}

watch(isAdult, (a) => {
  if (!a) selecting.value = false
})
</script>

<template>
  <UiPage wide>
    <template #top>
      <UiTopAppBar title="Food" />
    </template>

    <FoodTabs />

    <div v-if="isAdult" class="plan-actions">
      <UiButton icon="add" data-testid="plan-add" @click="addToday">Add meal</UiButton>
      <UiButton
        variant="tonal"
        icon="template"
        data-testid="plan-apply-template"
        @click="((applyError = null), (applyOpen = true))"
      >
        Apply template
      </UiButton>
      <UiButton
        variant="tonal"
        icon="cart"
        data-testid="plan-make-list"
        @click="((listError = null), (listOpen = true))"
      >
        Make grocery list
      </UiButton>
      <UiButton
        variant="text"
        icon="copy"
        data-testid="plan-save-template"
        @click="((templateError = null), (templateOpen = true))"
      >
        Save as template
      </UiButton>
    </div>

    <div v-if="selecting" class="plan-select" role="region" aria-label="Pick meals">
      <span>{{ selected.length }} selected</span>
      <UiButton variant="text" @click="((selecting = false), (selected = []))">Cancel</UiButton>
      <UiButton
        :disabled="selected.length === 0"
        :loading="listBusy"
        data-testid="make-list-selected"
        @click="makeList({ entry_ids: selected })"
      >
        Make list
      </UiButton>
    </div>

    <UiSkeleton v-if="!ready" :lines="6" />
    <UiErrorState
      v-else-if="store.error && !store.loaded"
      :message="store.error"
      @retry="store.load(from, addDays(from, weeks * 7 - 1))"
    />
    <template v-else>
      <UiButton variant="text" icon="arrow_upward" data-testid="plan-earlier" @click="earlier += 4">
        Show earlier weeks
      </UiButton>
      <MealPlanGrid
        :from="from"
        :weeks="weeks"
        :editable="isAdult"
        :slots="slots"
        :today="today"
        :max-date="maxDate"
        :selecting="selecting"
        :selected="selected"
        @add="openAdd"
        @select="onSelect"
        @toggle="toggleSelected"
        @drop-slot="dropSlot"
        @swap="swapEntries"
      />
    </template>

    <EntrySheet
      v-model="sheetOpen"
      :slots="slots"
      :date="sheetDate"
      :slot-id="sheetSlot"
      :entry="sheetEntry"
      :recipe="sheetRecipe"
      :min-date="today"
      :max-date="maxDate"
      :saving="sheetBusy"
      :error="sheetError"
      @save="saveEntry"
      @delete="deleteEntry"
    />
    <ApplyTemplateSheet
      v-model="applyOpen"
      :default-start="nextWeek"
      :min-date="today"
      :max-date="maxDate"
      :saving="applyBusy"
      :error="applyError"
      @apply="applyTemplate"
    />
    <MakeListSheet
      v-model="listOpen"
      :default-from="today"
      :saving="listBusy"
      :error="listError"
      @range="(f, t) => makeList({ from: f, to: t })"
      @pick="startPicking"
    />
    <SaveAsTemplateSheet
      v-model="templateOpen"
      :default-from="thisWeek"
      :saving="templateBusy"
      :error="templateError"
      @save="saveTemplate"
    />
    <ImpactDialog
      :items="store.impact"
      :busy="impactBusy"
      @update="updateList"
      @leave="store.dismissImpact()"
    />
  </UiPage>
</template>

<style scoped>
.plan-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.plan-select {
  position: sticky;
  top: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  padding: var(--ui-space-2) var(--ui-space-4);
  border-radius: var(--ui-radius-lg);
  background: var(--ui-surface-container-high);
}
.plan-select > span {
  flex: 1;
}
</style>
