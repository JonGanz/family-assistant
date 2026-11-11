<script setup lang="ts">
// The Food tab row (Frontend.md §3f): a UiTabs row. It reads the route
// and the role, remembers the tab per device (`fa.food.tab`) and is used by every Food view.
import { computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useSessionStore } from '../../stores/session'
import { UiTabs } from '../../ui'

const TABS = [
  { value: 'plan', label: 'Meal plan', adult: false },
  { value: 'list', label: 'Shopping list', adult: false },
  { value: 'recipes', label: 'Recipes', adult: false },
  { value: 'templates', label: 'Templates', adult: true },
]
const TAB_KEY = 'fa.food.tab'

const route = useRoute()
const router = useRouter()
const session = useSessionStore()

const tabs = computed(() => TABS.filter((t) => session.isAdult || !t.adult))
const current = computed(() => {
  const seg = route.path.split('/')[2] ?? 'plan'
  return tabs.value.some((t) => t.value === seg) ? seg : 'plan'
})

function remember(tab: string) {
  try {
    localStorage.setItem(TAB_KEY, tab)
  } catch {
    // Blocked storage: the tab is simply not remembered.
  }
}
// Opening a Food page directly also counts as choosing its tab.
watch(current, remember, { immediate: true })

function go(tab: string) {
  remember(tab)
  if (tab !== current.value || route.path !== `/food/${tab}`) void router.push(`/food/${tab}`)
}
</script>

<template>
  <UiTabs
    class="food-tabs"
    data-testid="food-tabs"
    label="Food"
    testid-prefix="food-tab"
    :model-value="current"
    :options="tabs.map((t) => ({ value: t.value, label: t.label, href: `/food/${t.value}` }))"
    @update:model-value="go"
  />
</template>
