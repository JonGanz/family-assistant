<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { food } from '../../api/food'
import { groupRecipe } from '../../lib/recipeSections'
import { useMealsSettingsStore } from '../../stores/mealsSettings'
import FoodTabs from '../../components/food/FoodTabs.vue'
import StarRating from '../../components/food/recipes/StarRating.vue'
import { useRecipesStore } from '../../stores/recipes'
import { useSessionStore } from '../../stores/session'
import {
  UiButton,
  UiCard,
  UiDialog,
  UiErrorState,
  UiIconButton,
  UiPage,
  UiSection,
  UiTopAppBar,
  useInstantSave,
} from '../../ui'

const route = useRoute()
const router = useRouter()
const store = useRecipesStore()
const session = useSessionStore()
const id = computed(() => String(route.params.id))
const confirmDelete = ref(false)
const confirmReimport = ref(false)
const r = computed(() => store.current)
const g = computed(() => groupRecipe(r.value ?? { lines: [], steps: [], sections: [] }))
const mealsSettings = useMealsSettingsStore()
const columns = computed(() => mealsSettings.settings?.recipe_section_columns ?? 2)

onMounted(() => {
  void store.open(id.value)
  if (!mealsSettings.loaded) void mealsSettings.load()
})

const mine = ref<number | null>(null)
const { set: setMine } = useInstantSave(
  mine,
  async (v) => {
    await store.rate(id.value, v)
  },
  { label: 'Rating saved' },
)
watch(
  () => r.value?.ratings.mine,
  (v) => (mine.value = v ?? null),
  { immediate: true },
)

async function toggleFav() {
  if (r.value) await store.favourite(r.value.id, !r.value.favourite)
}
async function del() {
  await store.remove(id.value)
  confirmDelete.value = false
  await router.push('/food/recipes')
}
async function reimport() {
  confirmReimport.value = false
  await router.push(`/food/recipes/${id.value}/edit?reimport=1`)
}
const addToPlan = () => router.push(`/food/plan?add=${id.value}`)
const mins = (n?: number | null) => (n ? `${n} min` : null)
</script>

<template>
  <UiPage>
    <template #top
      ><UiTopAppBar :title="r?.title ?? 'Recipe'" back @back="router.push('/food/recipes')"
    /></template>
    <FoodTabs />
    <UiErrorState v-if="store.error && !r" :message="store.error" @retry="store.open(id)" />
    <template v-else-if="r">
      <img v-if="r.has_image" :src="food.recipeImageUrl(r.id)" alt="" class="hero" />
      <div class="facts">
        <span v-if="r.servings">Serves {{ r.servings }}</span>
        <span v-if="mins(r.prep_minutes)">Prep {{ mins(r.prep_minutes) }}</span>
        <span v-if="mins(r.cook_minutes)">Cook {{ mins(r.cook_minutes) }}</span>
        <span v-if="mins(r.total_minutes)">Total {{ mins(r.total_minutes) }}</span>
        <span v-for="t in r.tags" :key="t" class="tag">{{ t }}</span>
      </div>
      <div class="actions">
        <UiButton icon="calendar" data-testid="recipe-add-plan" @click="addToPlan"
          >Add to plan</UiButton
        >
        <UiIconButton
          :icon="r.favourite ? 'star_filled' : 'star'"
          label="Favourite"
          :pressed="r.favourite"
          @click="toggleFav"
        />
        <template v-if="session.isAdult">
          <UiButton
            variant="tonal"
            icon="edit"
            data-testid="recipe-edit"
            :to="`/food/recipes/${r.id}/edit`"
            >Edit</UiButton
          >
          <UiButton
            v-if="r.source_url"
            variant="text"
            icon="refresh"
            data-testid="recipe-reimport"
            @click="confirmReimport = true"
            >Re-import from source</UiButton
          >
          <UiButton
            variant="text"
            icon="delete"
            data-testid="recipe-delete"
            @click="confirmDelete = true"
            >Delete</UiButton
          >
        </template>
      </div>
      <UiSection v-if="g.shared.length" heading="Ingredients">
        <ul class="lines" data-testid="recipe-shared">
          <li v-for="l in g.shared" :key="l.id" :data-testid="`recipe-line-${l.ord}`">
            {{ l.raw_text }}
            <span v-if="l.ingredient_name" class="matched">{{ l.ingredient_name }}</span>
          </li>
        </ul>
      </UiSection>
      <div
        v-if="g.sections.length"
        class="sections"
        :style="{ '--cols': Math.min(g.sections.length, columns) }"
        data-testid="recipe-sections"
      >
        <section
          v-for="sec in g.sections"
          :key="sec.section.id"
          class="sec"
          :data-testid="`recipe-section-${sec.section.ord}`"
        >
          <h2 class="sec__name">{{ sec.section.name }}</h2>
          <UiCard v-if="sec.lines.length" variant="filled">
            <ul class="lines">
              <li v-for="l in sec.lines" :key="l.id" :data-testid="`recipe-line-${l.ord}`">
                {{ l.raw_text }}
                <span v-if="l.ingredient_name" class="matched">{{ l.ingredient_name }}</span>
              </li>
            </ul>
          </UiCard>
          <UiCard v-if="sec.steps.length" variant="filled">
            <ol class="steps">
              <li v-for="s in sec.steps" :key="s.id">{{ s.text }}</li>
            </ol>
          </UiCard>
        </section>
      </div>
      <UiSection
        v-if="g.finish.length"
        :heading="g.sections.length ? 'Finish' : 'Steps'"
        data-testid="recipe-finish"
      >
        <ol class="steps">
          <li v-for="s in g.finish" :key="s.id">{{ s.text }}</li>
        </ol>
      </UiSection>
      <UiSection v-if="r.note" heading="Note"
        ><p>{{ r.note }}</p></UiSection
      >
      <UiSection heading="Ratings">
        <StarRating :model-value="mine" label="Your rating" @update:model-value="setMine" />
        <p v-if="r.ratings.avg" class="avg">
          Average {{ r.ratings.avg.toFixed(1) }} from {{ r.ratings.by_member.length }}
        </p>
      </UiSection>
      <p v-if="r.source_url" class="src">Source: {{ r.source_url }}</p>
    </template>
    <UiDialog v-model="confirmReimport" title="Re-import from source?">
      <p>
        Ingredients, steps and sections will be replaced by the page's current version. Nothing is
        saved until you press Save.
      </p>
      <template #actions>
        <UiButton variant="text" @click="confirmReimport = false">Cancel</UiButton>
        <UiButton data-testid="recipe-reimport-confirm" @click="reimport">Re-import</UiButton>
      </template>
    </UiDialog>
    <UiDialog v-model="confirmDelete" title="Delete this recipe?">
      <p>Planned meals keep the name as text.</p>
      <template #actions>
        <UiButton variant="text" @click="confirmDelete = false">Cancel</UiButton>
        <UiButton data-testid="recipe-delete-confirm" @click="del">Delete</UiButton>
      </template>
    </UiDialog>
  </UiPage>
</template>

<style scoped>
.hero {
  width: 100%;
  max-height: 320px;
  object-fit: cover;
  border-radius: var(--ui-radius-lg);
}
.facts,
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-3);
  align-items: center;
}
.tag {
  padding: var(--ui-space-1) var(--ui-space-2);
  border-radius: var(--ui-radius-md);
  background: var(--ui-secondary-container);
  color: var(--ui-on-secondary-container);
}
.lines,
.steps {
  margin: 0;
  padding-left: var(--ui-space-5);
  display: grid;
  gap: var(--ui-space-2);
}
.sections {
  display: grid;
  gap: var(--ui-space-4);
  grid-template-columns: 1fr;
}
@media (min-width: 600px) {
  .sections {
    grid-template-columns: repeat(var(--cols, 1), 1fr);
  }
}
.sec {
  display: grid;
  gap: var(--ui-space-3);
  align-content: start;
}
.sec__name {
  margin: 0;
}
.matched {
  display: none;
  color: var(--ui-on-surface-variant);
}
@media (min-width: 600px) {
  .matched {
    display: inline;
  }
  .matched::before {
    content: '\2192 ';
  }
}
.avg,
.src {
  color: var(--ui-on-surface-variant);
}
</style>
