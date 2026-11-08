<script setup lang="ts">
// A recipe (searched in the library) or free text, plus servings: the content of a plan entry or a template item.
import { onBeforeUnmount, ref, watch } from 'vue'
import { food, type FoodApi, type RecipeSummary } from '../../../api/food'
import { UiChip, UiSegmented, UiTextField } from '../../../ui'
import type { ItemPick } from './planModel'

const props = defineProps<{ modelValue: ItemPick; api?: FoodApi }>()
const emit = defineEmits<{ 'update:modelValue': [value: ItemPick] }>()

const query = ref('')
const results = ref<RecipeSummary[]>([])
const searching = ref(false)
const failed = ref(false)
let timer: ReturnType<typeof setTimeout> | undefined
let seq = 0

const patch = (p: Partial<ItemPick>) => emit('update:modelValue', { ...props.modelValue, ...p })

async function search() {
  const mine = ++seq
  searching.value = true
  try {
    const page = await (props.api ?? food).recipes({ q: query.value.trim(), limit: 8 })
    if (mine === seq) {
      results.value = page.items
      failed.value = false
    }
  } catch {
    if (mine === seq) failed.value = true
  } finally {
    if (mine === seq) searching.value = false
  }
}
watch(query, () => {
  clearTimeout(timer)
  timer = setTimeout(() => void search(), 250)
})
watch(
  () => [props.modelValue.kind, props.modelValue.recipe_id] as const,
  ([kind, id]) => {
    if (kind === 'recipe' && !id && results.value.length === 0) void search()
  },
  { immediate: true },
)
onBeforeUnmount(() => clearTimeout(timer))

function choose(r: RecipeSummary) {
  patch({ recipe_id: r.id, recipe_title: r.title })
  query.value = ''
}
</script>

<template>
  <div class="picker">
    <UiSegmented
      label="Kind of entry"
      :model-value="modelValue.kind"
      :options="[
        { value: 'recipe', label: 'Recipe' },
        { value: 'text', label: 'Free text' },
      ]"
      @update:model-value="patch({ kind: $event as ItemPick['kind'] })"
    />
    <template v-if="modelValue.kind === 'recipe'">
      <UiChip
        v-if="modelValue.recipe_id"
        variant="input"
        data-testid="pick-recipe"
        @remove="patch({ recipe_id: null, recipe_title: '' })"
      >
        {{ modelValue.recipe_title }}
      </UiChip>
      <template v-else>
        <UiTextField v-model="query" label="Search recipes" data-testid="pick-search" />
        <p v-if="failed" class="picker__note" role="alert">Could not search recipes.</p>
        <p v-else-if="!searching && results.length === 0" class="picker__note">No recipes found.</p>
        <ul v-else class="picker__list">
          <li v-for="r in results" :key="r.id">
            <button
              type="button"
              class="picker__item"
              :data-testid="`pick-result-${r.id}`"
              @click="choose(r)"
            >
              {{ r.title }}
            </button>
          </li>
        </ul>
      </template>
    </template>
    <UiTextField
      v-else
      :model-value="modelValue.text"
      label="What is planned"
      data-testid="pick-text"
      @update:model-value="patch({ text: $event })"
    />
    <UiTextField
      :model-value="modelValue.servings"
      type="number"
      label="Servings"
      supporting="Empty uses the recipe's servings."
      data-testid="pick-servings"
      @update:model-value="patch({ servings: $event })"
    />
  </div>
</template>

<style scoped>
.picker {
  display: grid;
  gap: var(--ui-space-3);
}
.picker__note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.picker__list {
  display: grid;
  margin: 0;
  padding: 0;
  list-style: none;
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-sm);
  overflow: hidden;
}
.picker__item {
  width: 100%;
  min-height: var(--ui-touch);
  padding: 0 var(--ui-space-4);
  border: 0;
  background: transparent;
  color: var(--ui-on-surface);
  font: var(--ui-type-body-lg);
  text-align: left;
  cursor: pointer;
}
.picker__item:hover {
  background: var(--ui-state-hover);
}
.picker__item:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: -2px;
}
</style>
