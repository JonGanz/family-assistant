<script setup lang="ts">
import { ref } from 'vue'
import { useMealsSettingsStore } from '../../../stores/mealsSettings'
import { UiCard, UiChip, UiSection, UiSwitch, UiTextField, useSnackbar } from '../../../ui'
import { describeError } from '../../shared'

const meals = useMealsSettingsStore()
const snackbar = useSnackbar()
const word = ref('')

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
const setHorizon = (v: string) => {
  const n = Math.round(Number(v))
  if (!(n >= 1 && n <= 52) || !s()) return
  void patch(
    { meal_plan_horizon_weeks: n },
    { meal_plan_horizon_weeks: s()!.meal_plan_horizon_weeks },
  )
}
const setThreshold = (v: string) => {
  const n = Number(v)
  if (!s() || !(n >= 0.2 && n <= 0.95)) return
  void patch({ trigram_threshold: n }, { trigram_threshold: s()!.trigram_threshold })
}
const setArchive = (v: string) => {
  const n = Math.round(Number(v))
  if (!(n >= 1 && n <= 720) || !s()) return
  void patch({ grocery_archive_hours: n }, { grocery_archive_hours: s()!.grocery_archive_hours })
}
const setLlm = (v: boolean) =>
  s() && patch({ ingredient_llm: v }, { ingredient_llm: s()!.ingredient_llm })
const setWords = (list: string[]) =>
  s() && patch({ descriptors: list }, { descriptors: s()!.descriptors })
function addWord() {
  const w = word.value.trim().toLowerCase()
  word.value = ''
  if (w && s() && !s()!.descriptors.includes(w)) void setWords([...s()!.descriptors, w])
}
</script>

<template>
  <UiSection heading="Planning and matching" supporting="Changes apply at once.">
    <UiCard v-if="meals.settings" variant="filled" data-testid="matching-card">
      <div class="stack">
        <UiTextField
          :model-value="String(meals.settings.meal_plan_horizon_weeks)"
          label="Plan horizon (weeks, 1 to 52)"
          type="number"
          name="horizon"
          min="1"
          max="52"
          @change="setHorizon(($event.target as HTMLInputElement).value)"
        />
        <UiSwitch
          :model-value="meals.settings.ingredient_llm"
          label="Ask the language model about ingredients the rules cannot match"
          @update:model-value="setLlm"
        />
        <label class="slider">
          <span>Match strictness: {{ meals.settings.trigram_threshold.toFixed(2) }}</span>
          <input
            type="range"
            min="0.2"
            max="0.95"
            step="0.05"
            name="threshold"
            :value="meals.settings.trigram_threshold"
            @change="setThreshold(($event.target as HTMLInputElement).value)"
          />
        </label>
        <div>
          <p class="label">Descriptor words (ignored when matching)</p>
          <div class="chips">
            <UiChip
              v-for="w in meals.settings.descriptors"
              :key="w"
              variant="input"
              @remove="setWords(meals.settings!.descriptors.filter((x) => x !== w))"
              >{{ w }}</UiChip
            >
          </div>
          <form @submit.prevent="addWord">
            <UiTextField v-model="word" label="Add a word" name="descriptor" />
          </form>
        </div>
        <UiTextField
          :model-value="String(meals.settings.grocery_archive_hours)"
          label="Keep checked items for (hours, 1 to 720)"
          type="number"
          name="archive"
          min="1"
          max="720"
          @change="setArchive(($event.target as HTMLInputElement).value)"
        />
      </div>
    </UiCard>
  </UiSection>
</template>

<style scoped>
.stack {
  display: grid;
  gap: var(--ui-space-4);
}
.slider {
  display: grid;
  gap: var(--ui-space-1);
}
.slider input {
  min-height: var(--ui-touch);
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
