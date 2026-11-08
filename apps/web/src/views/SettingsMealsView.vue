<script setup lang="ts">
import { onMounted } from 'vue'
import SettingsPage from '../app/SettingsPage.vue'
import { useMealsSettingsStore } from '../stores/mealsSettings'
import { UiSkeleton } from '../ui'
import PageNotices from './parts/settings-mail/PageNotices.vue'
import AislesCard from './parts/meals/AislesCard.vue'
import IngredientsCard from './parts/meals/IngredientsCard.vue'
import MatchingCard from './parts/meals/MatchingCard.vue'
import SlotsCard from './parts/meals/SlotsCard.vue'
import StoresCard from './parts/meals/StoresCard.vue'

const meals = useMealsSettingsStore()
onMounted(() => meals.load())
</script>

<template>
  <SettingsPage title="Meals">
    <PageNotices :load-error="meals.error" @reload="meals.load()" />
    <UiSkeleton v-if="!meals.loaded && !meals.error" :lines="4" />
    <div v-else-if="meals.loaded" class="page">
      <SlotsCard />
      <MatchingCard />
      <StoresCard />
      <AislesCard />
      <IngredientsCard />
    </div>
  </SettingsPage>
</template>

<style scoped>
.page {
  display: grid;
  gap: var(--ui-space-6);
}
</style>
