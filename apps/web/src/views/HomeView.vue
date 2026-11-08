<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import type { TaskView } from '../api/client'
import { addDays, dayKey } from '../lib/time'
import TodayMeals from '../components/food/TodayMeals.vue'
import { useCalendarStore } from '../stores/calendar'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import { useTasksStore } from '../stores/tasks'
import { useWeatherStore } from '../stores/weather'
import {
  UiCard,
  UiCheckbox,
  UiEmptyState,
  UiErrorState,
  UiList,
  UiPage,
  UiSection,
  UiSkeleton,
  UiTopAppBar,
  useSnackbar,
} from '../ui'
import AdvisoryList from './parts/AdvisoryList.vue'
import AgendaList from './parts/AgendaList.vue'
import WeatherCard from './parts/WeatherCard.vue'
import { describeError, memberLikes } from './shared'

const session = useSessionStore()
const household = useHouseholdStore()
const calendar = useCalendarStore()
const weather = useWeatherStore()
const members = useMembersStore()
const tasks = useTasksStore()
const snackbar = useSnackbar()

const tz = computed(() => household.tz)
const today = computed(() => dayKey(new Date(), tz.value))
const todayLabel = computed(() =>
  new Intl.DateTimeFormat(undefined, { timeZone: tz.value, dateStyle: 'full' }).format(new Date()),
)
const end = computed(() => addDays(today.value, 2))
const events = computed(() => calendar.eventsIn(today.value, end.value, tz.value))
const likes = computed(() => memberLikes(members.items))
const loadingEvents = computed(() => calendar.loading && events.value.length === 0)

const mineNow = computed<TaskView[]>(() => {
  const me = session.member?.id
  return (tasks.tabs.now ?? []).filter((t) => !!me && t.assignees.includes(me))
})

function loadRange() {
  void calendar.ensureRange(today.value, end.value, tz.value)
}

onMounted(() => {
  void weather.load()
  void members.load()
  void tasks.load('now')
  loadRange()
})
watch(tz, loadRange)

// Completing is instant (D-096): the store arms a 10-second undo and the snackbar offers it.
const UNDO_MS = 10_000
async function complete(t: TaskView) {
  try {
    await tasks.complete(t)
  } catch (e) {
    snackbar.show({ text: describeError(e, 'That did not work.') })
  }
}
watch(
  () => tasks.undo,
  (u) => {
    if (!u) return
    snackbar.show({
      text: `Completed “${u.title}”`,
      timeoutMs: UNDO_MS,
      action: {
        label: 'Undo',
        run: async () => {
          try {
            await tasks.undoLast()
          } catch (e) {
            snackbar.show({ text: describeError(e, 'That did not work.') })
          }
        },
      },
    })
    setTimeout(() => {
      if (tasks.undo?.id === u.id) tasks.expireUndo()
    }, UNDO_MS)
  },
)
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar :title="session.household?.name ?? 'Today'" />
    </template>

    <p class="date">{{ todayLabel }}</p>
    <AdvisoryList :advisories="weather.advisories" />
    <WeatherCard
      v-if="weather.latest"
      :weather="weather.latest"
      :tz="tz"
      :units="household.units"
    />
    <UiEmptyState
      v-else-if="weather.loaded"
      icon="weather"
      title="No weather yet"
      :text="
        session.isAdult
          ? 'Set the household location in Settings to enable it.'
          : 'An adult can set the household location in Settings.'
      "
    >
      <RouterLink v-if="session.isAdult" to="/settings/household">Household settings</RouterLink>
    </UiEmptyState>

    <UiSection v-if="mineNow.length" heading="Now">
      <UiCard variant="filled">
        <UiList aria-label="Now">
          <li v-for="t in mineNow" :key="t.id" class="now-row">
            <UiCheckbox :model-value="false" :label="t.title" @update:model-value="complete(t)" />
          </li>
        </UiList>
      </UiCard>
    </UiSection>

    <TodayMeals />

    <UiErrorState v-if="calendar.error" :message="calendar.error" @retry="loadRange" />
    <UiSkeleton v-else-if="loadingEvents" :lines="4" />
    <AgendaList v-else :events="events" :tz="tz" :from="today" :days="2" :members="likes" />
  </UiPage>
</template>

<style scoped>
.date {
  margin: 0;
  font: var(--ui-type-title-md);
  color: var(--ui-on-surface-variant);
}
.now-row {
  padding: var(--ui-space-1) var(--ui-space-3);
  min-height: var(--ui-touch);
  display: flex;
  align-items: center;
}
</style>
