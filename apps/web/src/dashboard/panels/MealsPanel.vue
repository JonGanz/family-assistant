<script setup lang="ts">
import { computed, ref } from 'vue'
import UiIcon from '../../ui/UiIcon.vue'
import RecipeDetail from '../details/RecipeDetail.vue'
import { mealsBySlot } from '../mealsModel'
import { S } from '../strings'
import type { PanelProps } from '../types'

// Local strings: strings.ts belongs to another task.
const T = { recipe: 'Recipe', openRecipe: 'Open recipe' }

const props = defineProps<PanelProps>()
const groups = computed(() => mealsBySlot(props.data))
/** The recipe open in the full-screen modal. The panel owns it: no canvas change is needed. */
const open = ref<{ id: string; title: string } | null>(null)
</script>

<template>
  <section class="meals" :aria-label="S.mealsToday" data-testid="panel-meals">
    <p v-if="groups.length === 0" class="none">{{ S.noMeals }}</p>
    <div v-else class="slots">
      <section v-for="g in groups" :key="g.slot_name" class="slot" data-testid="meal-slot">
        <h3 data-testid="meal-slot-title">{{ g.slot_name }}</h3>
        <ul>
          <li v-for="(m, i) in g.meals" :key="i" data-testid="meal-row">
            <span class="title">{{ m.title }}</span>
            <button
              v-if="m.recipe_id"
              type="button"
              class="recipe"
              data-testid="meal-recipe"
              :aria-label="`${T.openRecipe}: ${m.title}`"
              @click="open = { id: m.recipe_id, title: m.title }"
            >
              <UiIcon name="recipe" :size="24" />
              <span>{{ T.recipe }}</span>
            </button>
          </li>
        </ul>
      </section>
    </div>
    <RecipeDetail v-if="open" :id="open.id" :title="open.title" @close="open = null" />
  </section>
</template>

<style scoped>
h3 {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-lg);
  font-weight: 700;
}
.slots {
  display: grid;
  gap: var(--space-4);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}
li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--ui-radius-md, var(--ui-radius-sm));
  background: var(--color-layer-2, var(--color-surface));
  font-size: var(--font-size-lg);
}
.title {
  min-width: 0;
  overflow-wrap: anywhere;
}
.recipe {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  min-height: var(--min-touch);
  padding: 0 var(--space-3);
  border: 0;
  border-radius: var(--ui-radius-lg, var(--ui-radius-sm));
  background: var(--color-accent);
  color: var(--color-on-accent);
  font: inherit;
  font-size: var(--font-size-base);
  cursor: pointer;
}
.none {
  margin: 0;
  color: var(--color-muted);
}
</style>
