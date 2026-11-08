<script setup lang="ts">
import { ref } from 'vue'
import type { StoreAisleView, StoreView } from '../../../api/food'
import { useMealsSettingsStore } from '../../../stores/mealsSettings'
import { UiButton, UiCard, UiSection, UiSwitch, UiTextField, useSnackbar } from '../../../ui'
import { describeError } from '../../shared'
import ConfirmDialog from '../settings-mail/ConfirmDialog.vue'
import SheetForm from '../settings-mail/SheetForm.vue'
import { useConfirm } from '../settings-mail/useConfirm'
import MoveButtons from './MoveButtons.vue'
import { moved } from './reorder'

const meals = useMealsSettingsStore()
const snackbar = useSnackbar()
const confirm = useConfirm()
const editing = ref<number | 'new' | null>(null)
const name = ref('')
const aisleSheet = ref<StoreView | null>(null)
const order = ref<StoreAisleView[]>([])

async function apply(list: StoreView[]): Promise<boolean> {
  try {
    await meals.saveStores(list)
    snackbar.show({ text: 'Saved' })
    return true
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not save the stores.') })
    return false
  }
}
function open(i: number | 'new') {
  name.value = i === 'new' ? '' : (meals.stores[i]?.name ?? '')
  editing.value = i
}
async function save() {
  const n = name.value.trim()
  if (!n) return
  const list =
    editing.value === 'new'
      ? [...meals.stores, { name: n, ord: meals.stores.length }]
      : meals.stores.map((s, k) => (k === editing.value ? { ...s, name: n } : s))
  if (await apply(list)) editing.value = null
}
const remove = (i: number) =>
  confirm.ask(`Delete the store “${meals.stores[i]?.name}”?`, 'Items move to Any store.', () =>
    apply(meals.stores.filter((_, k) => k !== i)),
  )

const aisleName = (id: string) => meals.aisles.find((a) => a.id === id)?.name ?? 'Aisle'
async function openAisles(s: StoreView) {
  if (!s.id) return
  try {
    order.value = await meals.storeAisles(s.id)
    aisleSheet.value = s
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not load the aisle order.') })
  }
}
async function saveAisles() {
  const s = aisleSheet.value
  if (!s?.id) return
  try {
    await meals.saveStoreAisles(s.id, order.value)
    aisleSheet.value = null
    snackbar.show({ text: 'Saved' })
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not save the aisle order.') })
  }
}
</script>

<template>
  <UiSection heading="Stores" supporting="The shopping list groups by store, then by aisle.">
    <UiCard variant="filled" data-testid="stores-card">
      <ul class="rows">
        <li v-for="(s, i) in meals.stores" :key="s.id ?? i" class="row">
          <strong class="grow">{{ s.name }}</strong>
          <UiButton variant="text" @click="openAisles(s)">Aisle order</UiButton>
          <UiButton variant="text" @click="open(i)">Rename</UiButton>
          <UiButton variant="text" @click="remove(i)">Delete</UiButton>
          <MoveButtons
            :index="i"
            :count="meals.stores.length"
            :name="s.name"
            @move="apply(moved(meals.stores, i, $event))"
          />
        </li>
      </ul>
      <template #actions
        ><UiButton variant="tonal" @click="open('new')">Add a store</UiButton></template
      >
    </UiCard>
    <SheetForm
      :model-value="editing !== null"
      :title="editing === 'new' ? 'New store' : 'Rename store'"
      :save-disabled="!name.trim()"
      @update:model-value="editing = null"
      @save="save"
    >
      <UiTextField v-model="name" label="Name" name="store-name" required />
    </SheetForm>
    <SheetForm
      :model-value="!!aisleSheet"
      :title="`Aisle order: ${aisleSheet?.name ?? ''}`"
      @update:model-value="aisleSheet = null"
      @save="saveAisles"
    >
      <ul class="rows">
        <li v-for="(a, i) in order" :key="a.aisle_id" class="row">
          <span class="grow">{{ aisleName(a.aisle_id) }}</span>
          <UiSwitch
            :model-value="!a.hidden"
            :label="`This store has ${aisleName(a.aisle_id)}`"
            @update:model-value="a.hidden = !$event"
          />
          <MoveButtons
            :index="i"
            :count="order.length"
            :name="aisleName(a.aisle_id)"
            @move="order = moved(order, i, $event)"
          />
        </li>
      </ul>
    </SheetForm>
    <ConfirmDialog
      :pending="confirm.pending.value"
      @accept="confirm.accept"
      @cancel="confirm.cancel"
    />
  </UiSection>
</template>

<style scoped>
.rows {
  list-style: none;
  margin: 0;
  padding: 0;
}
.row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ui-space-1);
  min-height: var(--ui-touch);
}
.grow {
  flex: 1;
}
</style>
