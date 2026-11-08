<script setup lang="ts">
import { ref } from 'vue'
import { food, type RecipeLineView } from '../../../api/food'
import { useRecipesStore } from '../../../stores/recipes'
import { UiButton, UiTextField } from '../../../ui'

const props = defineProps<{ recipeId: string; line: RecipeLineView }>()
const store = useRecipesStore()
const mode = ref<'idle' | 'pick' | 'new'>('idle')
const q = ref('')
const found = ref<{ id: string; canonical_name: string }[]>([])
const newName = ref('')
const err = ref<string | null>(null)

async function run(b: { ingredient_id?: string; new_name?: string }) {
  err.value = null
  try {
    await store.matchLine(props.recipeId, props.line.id, b)
    mode.value = 'idle'
  } catch (e) {
    err.value = e instanceof Error ? e.message : 'Could not match the line'
  }
}
async function search() {
  found.value = q.value.trim() ? await food.ingredients(q.value.trim()) : []
}
</script>

<template>
  <div class="match" :class="`is-${line.match_state}`">
    <template v-if="mode === 'idle'">
      <span v-if="line.match_state === 'unmatched'" class="tag">No ingredient matched</span>
      <span v-if="line.proposal" class="chip" data-testid="recipe-proposal">
        Suggested: {{ line.proposal.new_name ?? 'catalogue ingredient' }}
      </span>
      <UiButton
        v-if="line.proposal"
        variant="tonal"
        data-testid="recipe-proposal-accept"
        @click="
          run(
            line.proposal.ingredient_id
              ? { ingredient_id: line.proposal.ingredient_id }
              : { new_name: line.proposal.new_name ?? undefined },
          )
        "
        >Accept</UiButton
      >
      <UiButton variant="text" @click="mode = 'pick'">Pick another</UiButton>
      <UiButton variant="text" @click="mode = 'new'">New ingredient</UiButton>
    </template>
    <template v-else-if="mode === 'pick'">
      <UiTextField v-model="q" label="Search ingredients" @update:model-value="search" />
      <UiButton
        v-for="i in found"
        :key="i.id"
        variant="text"
        @click="run({ ingredient_id: i.id })"
        >{{ i.canonical_name }}</UiButton
      >
      <UiButton variant="text" @click="mode = 'idle'">Cancel</UiButton>
    </template>
    <template v-else>
      <UiTextField v-model="newName" label="Ingredient name" />
      <UiButton :disabled="!newName.trim()" @click="run({ new_name: newName.trim() })"
        >Create and match</UiButton
      >
      <UiButton variant="text" @click="mode = 'idle'">Cancel</UiButton>
    </template>
    <p v-if="err" role="alert" class="err">{{ err }}</p>
  </div>
</template>

<style scoped>
.match {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
  align-items: center;
  padding: var(--ui-space-2);
  border-radius: var(--ui-radius-md);
  background: var(--ui-secondary-container);
  color: var(--ui-on-secondary-container);
}
.chip {
  font: var(--ui-type-label-lg);
}
.err {
  color: var(--ui-error);
}
</style>
