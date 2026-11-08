<script setup lang="ts">
import FoodTabs from '../../components/food/FoodTabs.vue'
import ShoppingList from '../../components/food/grocery/ShoppingList.vue'
import { useSessionStore } from '../../stores/session'
import { UiPage, UiTopAppBar } from '../../ui'

const session = useSessionStore()
</script>

<template>
  <UiPage>
    <template #top><UiTopAppBar title="Food" /></template>
    <FoodTabs />
    <ShoppingList :can-edit="session.isAdult" />
  </UiPage>
</template>
