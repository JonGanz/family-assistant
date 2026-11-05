<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import SettingsPage from '../app/SettingsPage.vue'
import { UiChip } from '../ui'
import AbsencesPanel from './parts/AbsencesPanel.vue'
import DefinitionsPanel from './parts/DefinitionsPanel.vue'
import RoutinesPanel from './parts/RoutinesPanel.vue'
import TaskListsPanel from './parts/TaskListsPanel.vue'

const SECTIONS = [
  { key: 'lists', label: 'Lists' },
  { key: 'definitions', label: 'Chores & schedules' },
  { key: 'routines', label: 'Routines' },
  { key: 'absences', label: 'Absences' },
] as const
type Key = (typeof SECTIONS)[number]['key']

const route = useRoute()
const router = useRouter()
const section = computed<Key>(() => {
  const q = route.query.tab
  return SECTIONS.some((t) => t.key === q) ? (q as Key) : 'lists'
})
const go = (key: Key) => router.replace({ query: { tab: key } })
</script>

<template>
  <SettingsPage title="Tasks">
    <div class="sections" role="group" aria-label="Task settings">
      <UiChip
        v-for="s in SECTIONS"
        :key="s.key"
        variant="filter"
        :selected="section === s.key"
        @click="go(s.key)"
        >{{ s.label }}</UiChip
      >
    </div>
    <TaskListsPanel v-if="section === 'lists'" />
    <DefinitionsPanel v-else-if="section === 'definitions'" />
    <RoutinesPanel v-else-if="section === 'routines'" />
    <AbsencesPanel v-else />
  </SettingsPage>
</template>

<style scoped>
.sections {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
</style>
