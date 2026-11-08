<script setup lang="ts">
import { computed, ref } from 'vue'
import type { MealSlotView } from '../../../api/food'
import { isSlotTime, useMealsSettingsStore } from '../../../stores/mealsSettings'
import { UiButton, UiCard, UiSection, UiSwitch, UiTextField, useSnackbar } from '../../../ui'
import { describeError } from '../../shared'
import SheetForm from '../settings-mail/SheetForm.vue'
import MoveButtons from './MoveButtons.vue'
import { moved } from './reorder'

const meals = useMealsSettingsStore()
const snackbar = useSnackbar()
const editing = ref<number | 'new' | null>(null)
const name = ref('')
const time = ref('18:00')
const error = ref('')
const timeError = computed(() =>
  isSlotTime(time.value) ? '' : 'Use 24-hour HH:MM, for example 18:00.',
)

async function apply(list: MealSlotView[], undo?: MealSlotView[]): Promise<boolean> {
  try {
    await meals.saveSlots(list)
    snackbar.show({
      text: 'Saved',
      ...(undo
        ? {
            action: {
              label: 'Undo',
              run: async () => {
                await meals.saveSlots(undo).catch(() => undefined)
              },
            },
          }
        : {}),
    })
    return true
  } catch (e) {
    error.value = describeError(e, 'Could not save the slots.')
    snackbar.show({ text: error.value })
    return false
  }
}
const toggle = (i: number, v: boolean) => {
  const before = meals.slots.map((s) => ({ ...s }))
  return apply(
    before.map((s, k) => (k === i ? { ...s, visible: v } : s)),
    before,
  )
}
const move = (i: number, d: number) =>
  apply(
    moved(meals.slots, i, d),
    meals.slots.map((s) => ({ ...s })),
  )
function open(i: number | 'new') {
  const s = i === 'new' ? null : meals.slots[i]
  name.value = s?.name ?? ''
  time.value = s?.time ?? '18:00'
  error.value = ''
  editing.value = i
}
async function save() {
  if (timeError.value || !name.value.trim()) return
  const before = meals.slots.map((s) => ({ ...s }))
  const list =
    editing.value === 'new'
      ? [
          ...before,
          { name: name.value.trim(), time: time.value, visible: true, ord: before.length },
        ]
      : before.map((s, k) =>
          k === editing.value ? { ...s, name: name.value.trim(), time: time.value } : s,
        )
  if (await apply(list)) editing.value = null
}
async function remove() {
  if (typeof editing.value !== 'number') return
  const before = meals.slots.map((s) => ({ ...s }))
  // The server answers `slot_in_use` while plan entries use the slot; show its message.
  if (await apply(before.filter((_, k) => k !== editing.value))) editing.value = null
}
</script>

<template>
  <UiSection heading="Meal slots" supporting="Only the shown slots appear in the plan.">
    <UiCard variant="filled" data-testid="slots-card">
      <ul class="rows">
        <li
          v-for="(s, i) in meals.slots"
          :key="s.id ?? i"
          class="row"
          :data-testid="`slot-${s.name}`"
        >
          <button type="button" class="row__main" @click="open(i)">
            <strong>{{ s.name }}</strong>
            <span class="meta">{{ s.time }}</span>
          </button>
          <UiSwitch
            :model-value="s.visible"
            :label="`Show ${s.name}`"
            @update:model-value="toggle(i, $event)"
          />
          <MoveButtons
            :index="i"
            :count="meals.slots.length"
            :name="s.name"
            @move="move(i, $event)"
          />
        </li>
      </ul>
      <template #actions
        ><UiButton variant="tonal" @click="open('new')">Add a slot</UiButton></template
      >
    </UiCard>
    <SheetForm
      :model-value="editing !== null"
      :title="editing === 'new' ? 'New slot' : 'Edit slot'"
      :save-disabled="!!timeError || !name.trim()"
      @update:model-value="editing = null"
      @save="save"
    >
      <UiTextField v-model="name" label="Name" name="slot-name" required />
      <UiTextField
        v-model="time"
        label="Time (HH:MM)"
        name="slot-time"
        :error="timeError"
        inputmode="numeric"
        required
      />
      <p v-if="error" role="alert">{{ error }}</p>
      <UiButton v-if="editing !== 'new'" variant="text" @click="remove">Remove this slot</UiButton>
    </SheetForm>
  </UiSection>
</template>

<style scoped>
.rows {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
}
.row {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  min-height: var(--ui-touch);
}
.row__main {
  flex: 1;
  display: grid;
  text-align: start;
  background: none;
  border: 0;
  color: inherit;
  font: inherit;
  min-height: var(--ui-touch);
  cursor: pointer;
}
.meta {
  color: var(--ui-on-surface-variant);
}
</style>
