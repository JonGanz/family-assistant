<script setup lang="ts">
import { ref } from 'vue'
import { useMealsSettingsStore } from '../../../stores/mealsSettings'
import { UiCard, UiChip, UiSection, UiSegmented, UiTextField, useSnackbar } from '../../../ui'
import { describeError } from '../../shared'

const meals = useMealsSettingsStore()
const snackbar = useSnackbar()
const factor = ref('')
const error = ref('')

async function patch(p: Parameters<typeof meals.saveSettings>[0], undo: typeof p) {
  try {
    await meals.saveSettings(p)
    snackbar.show({
      text: 'Saved',
      action: { label: 'Undo', run: () => meals.saveSettings(undo).then(() => undefined) },
    })
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not save.') })
  }
}
const s = () => meals.settings
const setFactors = (list: number[]) =>
  s() && patch({ recipe_scale_factors: list }, { recipe_scale_factors: s()!.recipe_scale_factors })
function addFactor() {
  const n = Number(factor.value.replace(',', '.'))
  if (!s()) return
  if (!factor.value.trim() || !(n >= 0.1 && n <= 10)) {
    error.value = 'Use a number from 0.1 to 10.'
    return
  }
  error.value = ''
  factor.value = ''
  const v = Math.round(n * 100) / 100
  const cur = s()!.recipe_scale_factors
  if (!cur.includes(v)) void setFactors([...cur, v].sort((a, b) => a - b))
}
const setColumns = (v: string) =>
  s() &&
  patch(
    { recipe_section_columns: Number(v) },
    { recipe_section_columns: s()!.recipe_section_columns },
  )
const columnOptions = [1, 2, 3, 4].map((n) => ({ value: String(n), label: String(n) }))
</script>

<template>
  <UiSection
    heading="Recipes"
    supporting="Scale buttons on the recipe window and how sections are laid out. Changes apply at once."
  >
    <UiCard v-if="meals.settings" variant="filled" data-testid="recipes-card">
      <div class="stack">
        <div>
          <p class="label">Scale factors (1 always stays)</p>
          <div class="chips" data-testid="scale-factors">
            <template v-for="f in meals.settings.recipe_scale_factors" :key="f">
              <UiChip v-if="f === 1" :data-testid="`scale-factor-${f}`">×1</UiChip>
              <UiChip
                v-else
                variant="input"
                :data-testid="`scale-factor-${f}`"
                @remove="setFactors(meals.settings!.recipe_scale_factors.filter((x) => x !== f))"
                >×{{ f }}</UiChip
              >
            </template>
          </div>
          <form @submit.prevent="addFactor">
            <UiTextField
              v-model="factor"
              label="Add a factor (0.1 to 10)"
              name="scale-factor"
              inputmode="decimal"
              :error="error"
            />
          </form>
        </div>
        <div>
          <p class="label">Section columns (at most, on wide screens)</p>
          <UiSegmented
            :model-value="String(meals.settings.recipe_section_columns)"
            :options="columnOptions"
            label="Section columns"
            data-testid="section-columns"
            @update:model-value="setColumns"
          />
        </div>
      </div>
    </UiCard>
  </UiSection>
</template>

<style scoped>
.stack {
  display: grid;
  gap: var(--ui-space-4);
}
.label {
  margin: 0 0 var(--ui-space-2);
  color: var(--ui-on-surface-variant);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
  margin-bottom: var(--ui-space-2);
}
</style>
