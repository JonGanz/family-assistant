<script setup lang="ts">
// Food → Templates → editor (adults): name, length in days and a grid of days x slots; each cell holds zero or more
// items (a recipe or free text) in order (D-112, D-120). One draft with one Save bar (D-096).
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { MealTemplateInput } from '../../api/food'
import ContentPicker from '../../components/food/plan/ContentPicker.vue'
import FoodTabs from '../../components/food/FoodTabs.vue'
import {
  emptyPick,
  pickFrom,
  pickPayload,
  pickValid,
  planSlots,
  type ItemPick,
} from '../../components/food/plan/planModel'
import { useMealPlanStore } from '../../stores/mealPlan'
import { useMealsSettingsStore } from '../../stores/mealsSettings'
import {
  UiButton,
  UiErrorState,
  UiIconButton,
  UiPage,
  UiSaveBar,
  UiSheet,
  UiSkeleton,
  UiTextField,
  UiTopAppBar,
  useFormDraft,
} from '../../ui'

interface FormCell {
  day: number
  slot_id: string
  pick: ItemPick
}
interface Form {
  name: string
  /** The input's text; validated on save (1 to 366). */
  days: string
  note: string
  cells: FormCell[]
}

const MAX_DAYS = 366

const route = useRoute()
const router = useRouter()
const store = useMealPlanStore()
const settings = useMealsSettingsStore()

const id = computed(() => String(route.params.id ?? 'new'))
const isNew = computed(() => id.value === 'new')
const version = ref(0)
const loading = ref(true)
const loadError = ref<string | null>(null)

const blank = (): Form => ({ name: '', days: '7', note: '', cells: [] })
const model = ref<Form>(blank())

const { draft, dirty, saving, error, save, discard } = useFormDraft<Form>(
  () => model.value,
  async (f) => {
    const days = Number.parseInt(f.days, 10)
    if (f.name.trim() === '') throw new Error('Give the template a name')
    if (!Number.isFinite(days) || days < 1 || days > MAX_DAYS)
      throw new Error(`Length must be 1 to ${MAX_DAYS} days`)
    // Order within a cell is the array order; items past the last day are dropped.
    const ord = new Map<string, number>()
    const cells = f.cells
      .filter((c) => c.day <= days && pickValid(c.pick))
      .map((c) => {
        const k = `${c.day}|${c.slot_id}`
        const o = ord.get(k) ?? 0
        ord.set(k, o + 1)
        return { day: c.day, slot_id: c.slot_id, ord: o, ...pickPayload(c.pick) }
      })
    const body: MealTemplateInput = {
      name: f.name.trim(),
      days,
      note: f.note.trim() || null,
      cells,
    }
    const v = await store.saveTemplate(isNew.value ? null : id.value, version.value, body)
    version.value = v.version
    model.value = toForm(v)
    if (isNew.value) await router.replace(`/food/templates/${v.id}`)
  },
)

function toForm(v: Awaited<ReturnType<typeof store.template>>): Form {
  const byOrd = [...v.cells]
  return {
    name: v.name,
    days: String(v.days),
    note: v.note ?? '',
    cells: byOrd.map((c) => ({ day: c.day, slot_id: c.slot_id, pick: pickFrom(c) })),
  }
}

async function load() {
  loading.value = true
  loadError.value = null
  try {
    if (!settings.loaded) await settings.load()
    if (isNew.value) {
      version.value = 0
      model.value = blank()
    } else {
      const v = await store.template(id.value)
      version.value = v.version
      model.value = toForm(v)
    }
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}
onMounted(load)
watch(id, (_n, o) => {
  // Saving a new template swaps `new` for its id: the draft already holds it, so there is nothing to load.
  if (o !== 'new') void load()
})

const dayCount = computed(() => {
  const n = Number.parseInt(draft.value.days, 10)
  return Number.isFinite(n) ? Math.min(Math.max(n, 0), MAX_DAYS) : 0
})
const days = computed(() => Array.from({ length: dayCount.value }, (_, i) => i + 1))
const weeks = computed(() => dayCount.value > 0 && dayCount.value % 7 === 0)
/** Visible slots, plus any hidden slot that already holds items (so nothing is silently invisible). */
const columns = computed(() => {
  const used = new Set(draft.value.cells.map((c) => c.slot_id))
  return planSlots(settings.slots.filter((s) => s.visible || (s.id && used.has(s.id))))
})
const itemsOf = (day: number, slotId: string) =>
  draft.value.cells
    .map((c, index) => ({ c, index }))
    .filter((x) => x.c.day === day && x.c.slot_id === slotId)

// --- The item sheet ------------------------------------------------------------------------------
const sheetOpen = ref(false)
const sheetPick = ref<ItemPick>(emptyPick())
/** The index being edited in `draft.cells`; null adds to (`addDay`, `addSlot`). */
const sheetIndex = ref<number | null>(null)
const addDay = ref(1)
const addSlot = ref('')

function openAdd(day: number, slotId: string) {
  sheetIndex.value = null
  addDay.value = day
  addSlot.value = slotId
  sheetPick.value = emptyPick()
  sheetOpen.value = true
}
function openEdit(index: number) {
  sheetIndex.value = index
  sheetPick.value = JSON.parse(JSON.stringify(draft.value.cells[index]!.pick)) as ItemPick
  sheetOpen.value = true
}
function commitItem() {
  if (!pickValid(sheetPick.value)) return
  if (sheetIndex.value === null)
    draft.value.cells.push({ day: addDay.value, slot_id: addSlot.value, pick: sheetPick.value })
  else draft.value.cells[sheetIndex.value]!.pick = sheetPick.value
  sheetOpen.value = false
}
function removeItem(index: number) {
  draft.value.cells.splice(index, 1)
}
/** Swap with the neighbouring item of the same cell. */
function shift(day: number, slotId: string, index: number, delta: -1 | 1) {
  const list = itemsOf(day, slotId)
  const at = list.findIndex((x) => x.index === index)
  const other = list[at + delta]
  if (!other) return
  const cells = draft.value.cells
  ;[cells[index], cells[other.index]] = [cells[other.index]!, cells[index]!]
}

const label = (p: ItemPick) =>
  (p.kind === 'recipe' ? p.recipe_title : p.text) + (p.servings ? ` (${p.servings})` : '')
</script>

<template>
  <UiPage wide>
    <template #top>
      <UiTopAppBar
        :title="isNew ? 'New template' : 'Edit template'"
        back
        back-label="Back to templates"
        @back="router.push('/food/templates')"
      />
    </template>

    <FoodTabs />

    <UiSkeleton v-if="loading" :lines="6" />
    <UiErrorState v-else-if="loadError" :message="loadError" @retry="load" />
    <form v-else class="tpl-edit" data-testid="template-editor" @submit.prevent="save">
      <div class="tpl-edit__head">
        <UiTextField v-model="draft.name" label="Name" required data-testid="template-name" />
        <UiTextField
          v-model="draft.days"
          type="number"
          label="Length in days"
          :supporting="`1 to ${MAX_DAYS}. Items beyond the last day are dropped.`"
          data-testid="template-days"
        />
      </div>
      <UiTextField v-model="draft.note" label="Note" multiline />

      <div class="grid" :class="{ 'is-weeks': weeks }">
        <section v-for="d in days" :key="d" class="day" :data-testid="`template-day-${d}`">
          <h3 class="day__head">
            Day {{ d }}<span v-if="weeks" class="day__week"> · week {{ Math.ceil(d / 7) }}</span>
          </h3>
          <div v-for="s in columns" :key="s.id" class="cell">
            <span v-if="columns.length > 1" class="cell__name">{{ s.name }}</span>
            <ul class="cell__items">
              <li v-for="x in itemsOf(d, s.id)" :key="x.index" class="item">
                <button
                  type="button"
                  class="item__label"
                  :data-testid="`template-item-${d}-${s.name.toLowerCase()}`"
                  @click="openEdit(x.index)"
                >
                  {{ label(x.c.pick) }}
                </button>
                <UiIconButton
                  icon="arrow_upward"
                  label="Move up"
                  :disabled="itemsOf(d, s.id)[0]?.index === x.index"
                  @click="shift(d, s.id, x.index, -1)"
                />
                <UiIconButton
                  icon="arrow_downward"
                  label="Move down"
                  :disabled="itemsOf(d, s.id).at(-1)?.index === x.index"
                  @click="shift(d, s.id, x.index, 1)"
                />
                <UiIconButton icon="delete" label="Remove item" @click="removeItem(x.index)" />
              </li>
            </ul>
            <button
              type="button"
              class="cell__add"
              :aria-label="`Add to day ${d}, ${s.name}`"
              :data-testid="`template-add-${d}-${s.name.toLowerCase()}`"
              @click="openAdd(d, s.id)"
            >
              +
            </button>
          </div>
        </section>
      </div>
    </form>

    <UiSaveBar :dirty="dirty" :saving="saving" :error="error" @save="save" @discard="discard" />

    <UiSheet v-model="sheetOpen" :title="sheetIndex === null ? 'Add an item' : 'Edit item'">
      <ContentPicker v-model="sheetPick" />
      <template #actions>
        <UiButton variant="text" @click="sheetOpen = false">Cancel</UiButton>
        <UiButton
          :disabled="!pickValid(sheetPick)"
          data-testid="template-item-save"
          @click="commitItem"
        >
          Done
        </UiButton>
      </template>
    </UiSheet>
  </UiPage>
</template>

<style scoped>
.tpl-edit {
  display: grid;
  gap: var(--ui-space-3);
}
.tpl-edit__head {
  display: grid;
  gap: var(--ui-space-3);
}
.grid {
  display: grid;
  gap: var(--ui-space-2);
  grid-template-columns: minmax(0, 1fr);
}
.day {
  display: grid;
  align-content: start;
  gap: var(--ui-space-2);
  min-width: 0;
  padding: var(--ui-space-2);
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-sm);
  background: var(--ui-surface-container-low);
}
.day__head {
  margin: 0;
  font: var(--ui-type-label-lg);
}
.day__week {
  color: var(--ui-on-surface-variant);
}
.cell {
  display: grid;
  gap: var(--ui-space-1);
  min-width: 0;
}
.cell__name {
  font: var(--ui-type-label-sm);
  color: var(--ui-on-surface-variant);
}
.cell__items {
  display: grid;
  gap: var(--ui-space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}
.item {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ui-space-1);
}
.item__label {
  flex: 1;
  min-width: 6rem;
  min-height: var(--ui-touch);
  padding: var(--ui-space-1) var(--ui-space-2);
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-sm);
  background: var(--ui-surface-container-high);
  color: var(--ui-on-surface);
  font: var(--ui-type-body-md);
  text-align: left;
  overflow-wrap: anywhere;
  cursor: pointer;
}
.cell__add {
  min-height: var(--ui-touch);
  border: 1px dashed var(--ui-outline);
  border-radius: var(--ui-radius-sm);
  background: transparent;
  color: inherit;
  font: var(--ui-type-title-md);
  cursor: pointer;
}
.item__label:focus-visible,
.cell__add:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 1px;
}
@media (min-width: 600px) {
  .tpl-edit__head {
    grid-template-columns: 2fr 1fr;
  }
  .grid.is-weeks {
    grid-template-columns: repeat(7, minmax(0, 1fr));
  }
}
</style>
