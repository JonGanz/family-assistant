<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { food, type FoodApi, type GroceryItemView } from '../../../api/food'
import { useGroceryStore } from '../../../stores/grocery'
import {
  UiButton,
  UiFilterChips,
  UiIcon,
  UiSelect,
  UiSheet,
  UiSkeleton,
  UiTextField,
  useSnackbar,
  type FilterChipOption,
} from '../../../ui'
import GroceryRow from './GroceryRow.vue'
import { groupItems } from './grouping'

const props = defineProps<{
  /** Adults (or an elevated kiosk): edit and delete. Everyone else only adds and checks. */
  canEdit: boolean
  /** Larger targets, no edit sheet. */
  kiosk?: boolean
  /** The kiosk passes its device-token API; the app uses the session one. */
  api?: FoodApi
}>()

const FILTER_KEY = 'fa.grocery.stores'
const store = useGroceryStore()
const snackbar = useSnackbar()
store.use(props.api ?? food)

function readFilter(): string[] {
  try {
    const v: unknown = JSON.parse(localStorage.getItem(FILTER_KEY) ?? '[]')
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []
  } catch {
    return []
  }
}
const chosen = ref<string[]>(readFilter())
watch(chosen, (v) => {
  try {
    localStorage.setItem(FILTER_KEY, JSON.stringify(v))
  } catch {
    /* remembered for this visit only */
  }
})
const storeOptions = computed<FilterChipOption[]>(() =>
  store.stores.filter((s) => s.id).map((s) => ({ value: s.id as string, label: s.name })),
)
// A store that no longer exists must not hide everything.
const activeFilter = computed(() =>
  chosen.value.filter((id) => storeOptions.value.some((o) => o.value === id)),
)

const groups = computed(() =>
  groupItems(store.open, store.stores, store.aisles, store.storeAisles, activeFilter.value),
)
const checkedItems = computed(() =>
  store.checked.filter(
    (i) =>
      !activeFilter.value.length ||
      !i.store_id ||
      activeFilter.value.includes(i.store_id as string) ||
      !storeOptions.value.some((o) => o.value === i.store_id),
  ),
)
const showChecked = ref(false)

onMounted(() => {
  void store.load()
})

// Add box with catalogue suggestions.
const text = ref('')
const suggestions = ref<string[]>([])
let timer: ReturnType<typeof setTimeout> | undefined
let seq = 0
watch(text, (q) => {
  clearTimeout(timer)
  const t = q.trim()
  if (t.length < 2) {
    suggestions.value = []
    return
  }
  timer = setTimeout(async () => {
    const mine = ++seq
    try {
      const got = await (props.api ?? food).ingredients(t)
      if (mine === seq)
        suggestions.value = got
          .map((i) => i.canonical_name)
          .filter((n) => n.toLowerCase() !== t.toLowerCase())
          .slice(0, 6)
    } catch {
      if (mine === seq) suggestions.value = []
    }
  }, 200)
})

async function addText(t: string) {
  const v = t.trim()
  if (!v) return
  text.value = ''
  suggestions.value = []
  try {
    await store.add(v)
  } catch (e) {
    snackbar.show({ text: e instanceof Error ? e.message : 'Could not add' })
  }
}

async function toggle(item: GroceryItemView, on: boolean) {
  try {
    if (on) {
      await store.check(item)
      snackbar.show({
        text: `Checked ${item.text}`,
        action: { label: 'Undo', run: () => store.uncheck(item) },
      })
    } else await store.uncheck(item)
  } catch (e) {
    snackbar.show({ text: e instanceof Error ? e.message : 'Could not save' })
  }
}

async function low(id: string) {
  const s = store.staples.find((x) => x.item_id === id)
  if (!s) return
  try {
    await store.low(s)
  } catch (e) {
    snackbar.show({ text: e instanceof Error ? e.message : 'Could not save' })
  }
}

function remove(item: GroceryItemView) {
  const undo = store.remove(item, () => snackbar.show({ text: `Could not delete ${item.text}` }))
  snackbar.show({ text: `Deleted ${item.text}`, action: { label: 'Undo', run: undo } })
}

// Edit sheet (adults, not on the kiosk).
const editing = ref<GroceryItemView | null>(null)
const form = ref({ text: '', qty: '', unit: '', store: '' })
const sheetOpen = computed({
  get: () => editing.value !== null,
  set: (v: boolean) => {
    if (!v) editing.value = null
  },
})
const formError = ref('')
function startEdit(item: GroceryItemView) {
  editing.value = item
  formError.value = ''
  form.value = {
    text: item.text,
    qty: item.qty != null ? String(item.qty) : '',
    unit: item.unit ?? '',
    store: (item.store_id as string | null | undefined) ?? '',
  }
}
async function saveEdit() {
  const item = editing.value
  if (!item) return
  const patch: Record<string, unknown> = {}
  const f = form.value
  if (f.text.trim() && f.text.trim() !== item.text) patch.text = f.text.trim()
  const qty = f.qty.trim() === '' ? null : Number(f.qty.replace(',', '.'))
  if (qty !== null && !Number.isFinite(qty)) {
    formError.value = 'Quantity must be a number'
    return
  }
  if (qty !== null && qty !== item.qty) patch.qty = qty
  if (f.unit.trim() !== (item.unit ?? '')) patch.unit = f.unit.trim()
  if (f.store && f.store !== item.store_id) patch.store_id = f.store
  if (!Object.keys(patch).length) {
    editing.value = null
    return
  }
  try {
    const ok = await store.update(item, patch)
    if (!ok) snackbar.show({ text: 'Someone else changed this item. The list is up to date.' })
    editing.value = null
  } catch (e) {
    formError.value = e instanceof Error ? e.message : 'Could not save'
  }
}
const storeSelect = computed(() => [
  { value: '', label: 'Any store' },
  ...storeOptions.value.map((o) => ({ value: o.value, label: o.label })),
])

const offlineText = computed(() => {
  const n = store.pending
  const w = `${n} change${n === 1 ? '' : 's'}`
  if (!store.online) return n ? `Offline, ${w} waiting` : 'Offline'
  return `Syncing ${w}`
})
const showNote = computed(() => !store.online || store.pending > 0)
const empty = computed(
  () => store.loaded && !store.open.length && !checkedItems.value.length && !store.staples.length,
)
</script>

<template>
  <div class="shopping" :class="{ 'is-kiosk': kiosk }">
    <form class="shopping__add" @submit.prevent="addText(text)">
      <UiTextField
        v-model="text"
        label="Add an item"
        data-testid="grocery-add-input"
        autocomplete="off"
        enterkeyhint="done"
      />
      <UiButton type="submit" icon="add" :disabled="!text.trim()">Add</UiButton>
    </form>
    <div v-if="suggestions.length" class="shopping__suggest" role="list">
      <button
        v-for="s in suggestions"
        :key="s"
        class="shopping__suggestion"
        type="button"
        role="listitem"
        @click="addText(s)"
      >
        <UiIcon name="add" :size="20" /> {{ s }}
      </button>
    </div>

    <p v-if="showNote" class="shopping__note" role="status" data-testid="grocery-offline-note">
      <UiIcon name="offline" :size="20" /> {{ offlineText }}
    </p>

    <UiFilterChips
      v-if="storeOptions.length"
      v-model="chosen"
      :options="storeOptions"
      all-label="All stores"
      label="Stores"
    />

    <UiSkeleton v-if="!store.loaded" />
    <p v-else-if="store.error && !store.view" class="shopping__empty" role="alert">
      Could not load the list. {{ store.error }}
    </p>

    <template v-else>
      <section
        v-if="store.staples.length"
        class="shopping__staples"
        data-testid="grocery-staples"
        aria-labelledby="grocery-staples-h"
      >
        <h2 id="grocery-staples-h">Staples check</h2>
        <p class="shopping__sub">Running low on any of these?</p>
        <ul>
          <li v-for="s in store.staples" :key="s.item_id">
            <span class="shopping__staple-name">{{ s.name }}</span>
            <UiButton
              variant="tonal"
              :data-testid="`grocery-low-${s.item_id}`"
              :aria-label="`${s.name} is low`"
              @click="low(s.item_id)"
              >Low</UiButton
            >
          </li>
        </ul>
      </section>

      <section v-for="g in groups" :key="g.storeId ?? 'any'" class="shopping__store">
        <h2 v-if="g.name" class="shopping__store-h">
          <UiIcon name="store" :size="20" /> {{ g.name }}
        </h2>
        <div v-for="a in g.aisles" :key="a.aisleId ?? 'other'" class="shopping__aisle">
          <h3 v-if="g.aisles.length > 1 || a.aisleId">{{ a.name }}</h3>
          <ul>
            <GroceryRow
              v-for="i in a.items"
              :key="i.id"
              :item="i"
              :kiosk="kiosk"
              :editable="canEdit && !kiosk"
              :deletable="canEdit"
              :offline="!store.online"
              @toggle="toggle(i, $event)"
              @edit="startEdit(i)"
              @remove="remove(i)"
            />
          </ul>
        </div>
      </section>

      <p v-if="empty" class="shopping__empty">Nothing on the list.</p>

      <section v-if="checkedItems.length" class="shopping__checked">
        <button
          class="shopping__checked-h"
          type="button"
          :aria-expanded="showChecked"
          data-testid="grocery-checked-toggle"
          @click="showChecked = !showChecked"
        >
          <UiIcon name="check" :size="20" /> Checked ({{ checkedItems.length }})
        </button>
        <ul v-if="showChecked">
          <GroceryRow
            v-for="i in checkedItems"
            :key="i.id"
            :item="i"
            :kiosk="kiosk"
            :editable="false"
            :deletable="canEdit"
            :offline="!store.online"
            @toggle="toggle(i, $event)"
            @remove="remove(i)"
          />
        </ul>
      </section>
    </template>

    <UiSheet v-if="canEdit && !kiosk" v-model="sheetOpen" title="Edit item">
      <form class="shopping__form" @submit.prevent="saveEdit">
        <UiTextField v-model="form.text" label="Item" data-testid="grocery-edit-text" />
        <UiTextField v-model="form.qty" label="Quantity" inputmode="decimal" />
        <UiTextField v-model="form.unit" label="Unit" />
        <UiSelect
          v-if="storeOptions.length"
          v-model="form.store"
          label="Store"
          :options="storeSelect"
        />
        <p v-if="formError" class="shopping__error" role="alert">{{ formError }}</p>
        <button type="submit" hidden />
      </form>
      <template #actions>
        <UiButton variant="text" @click="sheetOpen = false">Cancel</UiButton>
        <UiButton data-testid="grocery-edit-save" @click="saveEdit">Save</UiButton>
      </template>
    </UiSheet>
  </div>
</template>

<style scoped>
.shopping {
  display: grid;
  gap: var(--ui-space-4);
}
.shopping__add {
  display: flex;
  align-items: flex-start;
  gap: var(--ui-space-2);
}
.shopping__add > :first-child {
  flex: 1;
  min-width: 0;
}
.shopping__suggest {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.shopping__suggestion {
  display: inline-flex;
  align-items: center;
  gap: var(--ui-space-1);
  min-height: var(--ui-touch);
  padding: 0 var(--ui-space-3);
  border: 1px solid var(--ui-outline);
  border-radius: var(--ui-radius-sm);
  background: transparent;
  color: var(--ui-on-surface);
  font: var(--ui-type-label-lg);
  cursor: pointer;
}
.shopping__note {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  margin: 0;
  padding: var(--ui-space-2) var(--ui-space-3);
  border-radius: var(--ui-radius-sm);
  background: var(--ui-surface-container-high);
  color: var(--ui-on-surface);
  font: var(--ui-type-body-md);
}
h2,
h3 {
  margin: 0;
}
.shopping__store-h,
.shopping__staples h2 {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  font: var(--ui-type-title-md);
  color: var(--ui-on-surface);
}
.shopping__aisle h3 {
  padding: var(--ui-space-2) 0 var(--ui-space-1);
  font: var(--ui-type-label-lg);
  color: var(--ui-on-surface-variant);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
}
.shopping__store {
  display: grid;
  gap: var(--ui-space-1);
}
.shopping__staples {
  padding: var(--ui-space-3);
  border-radius: var(--ui-radius-md);
  background: var(--ui-surface-container-low);
}
.shopping__sub {
  margin: 0 0 var(--ui-space-2);
  font: var(--ui-type-body-md);
  color: var(--ui-on-surface-variant);
}
.shopping__staples li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ui-space-3);
  min-height: var(--ui-row-height);
}
.shopping__staple-name {
  font: var(--ui-type-body-lg);
  color: var(--ui-on-surface);
}
.shopping__empty {
  margin: 0;
  text-align: center;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-lg);
}
.shopping__checked-h {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  min-height: var(--ui-touch);
  padding: 0;
  border: 0;
  background: none;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-title-sm);
  cursor: pointer;
}
.shopping__form {
  display: grid;
  gap: var(--ui-space-3);
}
.shopping__error {
  margin: 0;
  color: var(--ui-error);
  font: var(--ui-type-body-md);
}
.is-kiosk .shopping__suggestion {
  min-height: 56px;
}
@media (min-width: 840px) {
  .shopping__store {
    gap: var(--ui-space-2);
  }
}
</style>
