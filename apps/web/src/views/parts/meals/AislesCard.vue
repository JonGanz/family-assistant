<script setup lang="ts">
import { ref } from 'vue'
import type { AisleView } from '../../../api/food'
import { useMealsSettingsStore } from '../../../stores/mealsSettings'
import { UiButton, UiCard, UiSection, UiTextField, useSnackbar } from '../../../ui'
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

async function apply(list: AisleView[]): Promise<boolean> {
  try {
    await meals.saveAisles(list)
    snackbar.show({ text: 'Saved' })
    return true
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not save the aisles.') })
    return false
  }
}
function open(i: number | 'new') {
  name.value = i === 'new' ? '' : (meals.aisles[i]?.name ?? '')
  editing.value = i
}
async function save() {
  const n = name.value.trim()
  if (!n) return
  const list =
    editing.value === 'new'
      ? [...meals.aisles, { name: n, ord: meals.aisles.length }]
      : meals.aisles.map((a, k) => (k === editing.value ? { ...a, name: n } : a))
  if (await apply(list)) editing.value = null
}
const remove = (i: number) =>
  confirm.ask(
    `Delete the aisle “${meals.aisles[i]?.name}”?`,
    'Ingredients in it go to no aisle.',
    () => apply(meals.aisles.filter((_, k) => k !== i)),
  )
</script>

<template>
  <UiSection heading="Aisles" supporting="The household order, used for Any store and new stores.">
    <UiCard variant="filled" data-testid="aisles-card">
      <ul class="rows">
        <li v-for="(a, i) in meals.aisles" :key="a.id ?? i" class="row">
          <strong class="grow">{{ a.name }}</strong>
          <UiButton variant="text" @click="open(i)">Rename</UiButton>
          <UiButton variant="text" @click="remove(i)">Delete</UiButton>
          <MoveButtons
            :index="i"
            :count="meals.aisles.length"
            :name="a.name"
            @move="apply(moved(meals.aisles, i, $event))"
          />
        </li>
      </ul>
      <template #actions
        ><UiButton variant="tonal" @click="open('new')">Add an aisle</UiButton></template
      >
    </UiCard>
    <SheetForm
      :model-value="editing !== null"
      :title="editing === 'new' ? 'New aisle' : 'Rename aisle'"
      :save-disabled="!name.trim()"
      @update:model-value="editing = null"
      @save="save"
    >
      <UiTextField v-model="name" label="Name" name="aisle-name" required />
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
