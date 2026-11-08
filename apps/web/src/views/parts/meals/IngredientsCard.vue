<script setup lang="ts">
import { computed, ref } from 'vue'
import type { IngredientInput, IngredientView } from '../../../api/food'
import { useMealsSettingsStore } from '../../../stores/mealsSettings'
import {
  UiButton,
  UiCard,
  UiChip,
  UiSection,
  UiSelect,
  UiSwitch,
  UiTextField,
  useSnackbar,
} from '../../../ui'
import { describeError } from '../../shared'
import ConfirmDialog from '../settings-mail/ConfirmDialog.vue'
import SheetForm from '../settings-mail/SheetForm.vue'
import { useConfirm } from '../settings-mail/useConfirm'

const meals = useMealsSettingsStore()
const snackbar = useSnackbar()
const confirm = useConfirm()
const q = ref('')
const editing = ref<string | 'new' | null>(null)
const mergeInto = ref('')
const alias = ref('')
const error = ref('')
const form = ref<IngredientInput>(blank())

function blank(): IngredientInput {
  return {
    canonical_name: '',
    aliases: [],
    is_staple: false,
    aisle_id: null,
    default_store_id: null,
    default_unit: null,
  }
}
const shown = computed(() => {
  const t = q.value.trim().toLowerCase()
  if (!t) return meals.ingredients
  return meals.ingredients.filter(
    (i) =>
      i.canonical_name.toLowerCase().includes(t) ||
      i.aliases.some((a) => a.toLowerCase().includes(t)),
  )
})
const toInput = (i: IngredientView): IngredientInput => ({
  canonical_name: i.canonical_name,
  aliases: [...i.aliases],
  is_staple: i.is_staple,
  aisle_id: i.aisle_id ?? null,
  default_store_id: i.default_store_id ?? null,
  default_unit: i.default_unit ?? null,
})
const aisleOptions = computed(() => [
  { value: '', label: 'No aisle' },
  ...meals.aisles.map((a) => ({ value: a.id ?? '', label: a.name })),
])
const storeOptions = computed(() => [
  { value: '', label: 'Any store' },
  ...meals.stores.map((s) => ({ value: s.id ?? '', label: s.name })),
])
const unitOptions = [
  { value: '', label: 'None' },
  { value: 'g', label: 'Grams' },
  { value: 'ml', label: 'Millilitres' },
  { value: 'count', label: 'Count' },
]
const mergeOptions = computed(() =>
  meals.ingredients
    .filter((i) => i.id !== editing.value)
    .map((i) => ({ value: i.id, label: i.canonical_name })),
)

async function setStaple(i: IngredientView, v: boolean) {
  const before = toInput(i)
  try {
    await meals.updateIngredient(i.id, { ...before, is_staple: v })
    snackbar.show({
      text: 'Saved',
      action: {
        label: 'Undo',
        run: () => meals.updateIngredient(i.id, before).then(() => undefined),
      },
    })
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not save.') })
  }
}
function open(i: IngredientView | null) {
  form.value = i ? toInput(i) : blank()
  editing.value = i ? i.id : 'new'
  mergeInto.value = ''
  alias.value = ''
  error.value = ''
}
function addAlias() {
  const a = alias.value.trim()
  alias.value = ''
  if (a && !form.value.aliases.includes(a)) form.value.aliases.push(a)
}
async function save() {
  if (!form.value.canonical_name.trim()) return
  error.value = ''
  try {
    const b = { ...form.value, canonical_name: form.value.canonical_name.trim() }
    if (editing.value === 'new') await meals.createIngredient(b)
    else if (editing.value) await meals.updateIngredient(editing.value, b)
    editing.value = null
  } catch (e) {
    error.value = describeError(e, 'Save failed.')
  }
}
const askMerge = () => {
  const from = meals.ingredients.find((i) => i.id === editing.value)
  const into = meals.ingredients.find((i) => i.id === mergeInto.value)
  if (!from || !into) return
  confirm.ask(
    `Merge “${from.canonical_name}” into “${into.canonical_name}”?`,
    'Recipe lines and aliases move over, and this ingredient is removed.',
    async () => {
      try {
        await meals.merge(from.id, into.id)
        editing.value = null
      } catch (e) {
        error.value = describeError(e, 'Merge failed.')
      }
    },
    'Merge',
  )
}
const askDelete = () => {
  const i = meals.ingredients.find((x) => x.id === editing.value)
  if (!i) return
  confirm.ask(`Delete “${i.canonical_name}”?`, 'It must not be used by a recipe.', async () => {
    try {
      await meals.deleteIngredient(i.id)
      editing.value = null
    } catch (e) {
      error.value = describeError(e, 'Could not delete.')
    }
  })
}
</script>

<template>
  <UiSection
    heading="Ingredients"
    supporting="Staples are assumed to be at home, so they are left off generated lists."
  >
    <UiCard variant="filled" data-testid="ingredients-card">
      <UiTextField v-model="q" label="Search ingredients" name="ingredient-search" type="search" />
      <ul class="rows">
        <li
          v-for="i in shown"
          :key="i.id"
          class="row"
          :data-testid="`ingredient-${i.canonical_name}`"
        >
          <button type="button" class="row__main" @click="open(i)">{{ i.canonical_name }}</button>
          <UiSwitch
            :model-value="i.is_staple"
            :label="`${i.canonical_name} is a staple`"
            @update:model-value="setStaple(i, $event)"
          />
        </li>
      </ul>
      <template #actions
        ><UiButton variant="tonal" @click="open(null)">New ingredient</UiButton></template
      >
    </UiCard>
    <SheetForm
      :model-value="editing !== null"
      :title="editing === 'new' ? 'New ingredient' : 'Edit ingredient'"
      :save-disabled="!form.canonical_name.trim()"
      @update:model-value="editing = null"
      @save="save"
    >
      <UiTextField v-model="form.canonical_name" label="Name" name="canonical" required />
      <div class="chips">
        <UiChip
          v-for="a in form.aliases"
          :key="a"
          variant="input"
          @remove="form.aliases = form.aliases.filter((x) => x !== a)"
          >{{ a }}</UiChip
        >
      </div>
      <UiTextField
        v-model="alias"
        label="Add an alias (press Enter)"
        name="alias"
        @keydown.enter.prevent="addAlias"
      />
      <UiSelect
        :model-value="form.aisle_id ?? ''"
        label="Aisle"
        :options="aisleOptions"
        @update:model-value="form.aisle_id = $event || null"
      />
      <UiSelect
        :model-value="form.default_store_id ?? ''"
        label="Default store"
        :options="storeOptions"
        @update:model-value="form.default_store_id = $event || null"
      />
      <UiSelect
        :model-value="form.default_unit ?? ''"
        label="Default unit"
        :options="unitOptions"
        @update:model-value="
          form.default_unit = ($event || null) as IngredientInput['default_unit']
        "
      />
      <UiSwitch v-model="form.is_staple" label="Staple" />
      <p v-if="error" role="alert">{{ error }}</p>
      <template v-if="editing !== 'new'">
        <UiSelect
          v-model="mergeInto"
          label="Merge into…"
          :options="[{ value: '', label: 'Choose…' }, ...mergeOptions]"
        />
        <UiButton variant="tonal" :disabled="!mergeInto" @click="askMerge">Merge</UiButton>
        <UiButton variant="text" @click="askDelete">Delete this ingredient</UiButton>
      </template>
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
  max-height: 24rem;
  overflow: auto;
}
.row {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  min-height: var(--ui-touch);
}
.row__main {
  flex: 1;
  text-align: start;
  background: none;
  border: 0;
  color: inherit;
  font: inherit;
  min-height: var(--ui-touch);
  cursor: pointer;
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
</style>
