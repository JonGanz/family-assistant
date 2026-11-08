<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { food } from '../../api/food'
import FoodTabs from '../../components/food/FoodTabs.vue'
import NewRecipeSheet from '../../components/food/recipes/NewRecipeSheet.vue'
import { useRecipesStore } from '../../stores/recipes'
import { useSessionStore } from '../../stores/session'
import {
  UiCard,
  UiEmptyState,
  UiErrorState,
  UiFab,
  UiFilterChips,
  UiIcon,
  UiPage,
  UiTextField,
  UiTopAppBar,
} from '../../ui'

const store = useRecipesStore()
const session = useSessionStore()
const sheet = ref(false)
const allTags = ref<string[]>([])

const FAV = '★favourites'
const chips = computed(() => [
  { value: FAV, label: 'Favourites', icon: 'star' as const },
  ...allTags.value.map((t) => ({ value: t, label: t })),
])
const selected = computed({
  get: () => [...(store.query.favourite ? [FAV] : []), ...store.query.tags],
  set: (v: string[]) => {
    store.query.favourite = v.includes(FAV)
    store.query.tags = v.filter((x) => x !== FAV)
  },
})
const shown = computed(() =>
  store.items.filter((r) => store.query.tags.every((t) => r.tags.includes(t))),
)

onMounted(async () => {
  void store.load()
  try {
    allTags.value = await food.recipeTags()
  } catch {
    allTags.value = []
  }
})
let timer: ReturnType<typeof setTimeout> | undefined
watch(
  () => [store.query.q, store.query.tags[0], store.query.favourite],
  () => {
    clearTimeout(timer)
    timer = setTimeout(() => void store.load(), 250)
  },
)

const mins = (n?: number | null) => (n ? `${n} min` : '')
</script>

<template>
  <UiPage wide>
    <template #top><UiTopAppBar title="Food" /></template>
    <FoodTabs />
    <UiTextField v-model="store.query.q" label="Search title or ingredient" type="search" />
    <UiFilterChips v-model="selected" :options="chips" label="Recipe tags" />
    <UiErrorState v-if="store.error" :message="store.error" @retry="store.load()" />
    <UiEmptyState v-else-if="store.loaded && !shown.length" title="No recipes yet" icon="recipe" />
    <ul v-else class="grid">
      <li v-for="r in shown" :key="r.id">
        <UiCard
          variant="outlined"
          :to="`/food/recipes/${r.id}`"
          :data-testid="`recipe-card-${r.id}`"
        >
          <template v-if="r.has_image" #media>
            <img :src="food.recipeImageUrl(r.id)" alt="" loading="lazy" class="img" />
          </template>
          <template #header
            ><strong>{{ r.title }}</strong></template
          >
          <div class="meta">
            <span v-if="r.total_minutes">{{ mins(r.total_minutes) }}</span>
            <span v-if="r.rating_avg"
              ><UiIcon name="star_filled" :size="20" /> {{ r.rating_avg.toFixed(1) }}</span
            >
            <span v-if="r.last_planned">Last planned {{ r.last_planned }}</span>
          </div>
        </UiCard>
      </li>
    </ul>
    <button v-if="store.nextCursor" class="more" type="button" @click="store.more()">
      Show more
    </button>
    <UiFab
      v-if="session.isAdult"
      icon="add"
      label="New recipe"
      data-testid="recipe-new"
      @click="sheet = true"
    />
    <NewRecipeSheet v-model="sheet" />
  </UiPage>
</template>

<style scoped>
.grid {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--ui-space-3);
  grid-template-columns: 1fr;
}
@media (min-width: 600px) {
  .grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
@media (min-width: 840px) {
  .grid {
    grid-template-columns: repeat(3, 1fr);
  }
}
.img {
  width: 100%;
  aspect-ratio: 4 / 3;
  object-fit: cover;
  display: block;
}
.meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-3);
  font: var(--ui-type-body-md);
  color: var(--ui-on-surface-variant);
}
.more {
  min-height: 48px;
  background: none;
  border: 0;
  color: var(--ui-primary);
  cursor: pointer;
}
</style>
