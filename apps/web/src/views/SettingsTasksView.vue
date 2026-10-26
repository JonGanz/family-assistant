<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import AbsencesPanel from './parts/AbsencesPanel.vue'
import DefinitionsPanel from './parts/DefinitionsPanel.vue'
import RoutinesPanel from './parts/RoutinesPanel.vue'
import TaskListsPanel from './parts/TaskListsPanel.vue'
import PageHeader from '../components/PageHeader.vue'

const TABS = [
  { key: 'lists', label: 'Lists' },
  { key: 'definitions', label: 'Chores & schedules' },
  { key: 'routines', label: 'Routines' },
  { key: 'absences', label: 'Absences' },
] as const
type Key = (typeof TABS)[number]['key']

const route = useRoute()
const router = useRouter()
const tab = computed<Key>(() => {
  const q = route.query.tab
  return TABS.some((t) => t.key === q) ? (q as Key) : 'lists'
})
const go = (key: Key) => router.replace({ query: { tab: key } })
</script>

<template>
  <section>
    <PageHeader eyebrow="Settings" title="Tasks" />
    <div class="tabs" role="tablist">
      <button
        v-for="t in TABS"
        :key="t.key"
        type="button"
        role="tab"
        :aria-selected="tab === t.key"
        @click="go(t.key)"
      >
        {{ t.label }}
      </button>
    </div>
    <TaskListsPanel v-if="tab === 'lists'" />
    <DefinitionsPanel v-else-if="tab === 'definitions'" />
    <RoutinesPanel v-else-if="tab === 'routines'" />
    <AbsencesPanel v-else />
  </section>
</template>
