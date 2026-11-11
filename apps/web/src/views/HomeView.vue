<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import type { TaskView } from '../api/client'
import { addDays, dayKey, groupByDay } from '../lib/time'
import { food, type MealPlanEntryView } from '../api/food'
import { useCalendarStore } from '../stores/calendar'
import { useHouseholdStore } from '../stores/household'
import { useMealsSettingsStore } from '../stores/mealsSettings'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import { useTasksStore } from '../stores/tasks'
import { useWeatherStore } from '../stores/weather'
import { UiEmptyState, UiErrorState, UiPage, UiSkeleton, UiTopAppBar, useSnackbar } from '../ui'
import AdvisoryList from './parts/AdvisoryList.vue'
import TodayDay from './today/TodayDay.vue'
import TodayWeather from './today/TodayWeather.vue'
import { mealCards, tasksDueOn } from './today/todayModel'
import { describeError, memberLikes, toEventLike } from './shared'

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
const tomorrow = computed(() => addDays(today.value, 1))
const end = computed(() => addDays(today.value, 2))
const events = computed(() => calendar.eventsIn(today.value, end.value, tz.value))
const likes = computed(() => memberLikes(members.items))
const loadingEvents = computed(() => calendar.loading && events.value.length === 0)
const byDay = computed(() =>
  groupByDay(
    events.value.map((e) => toEventLike(e, likes.value)),
    tz.value,
    today.value,
    2,
  ),
)
const eventsToday = computed(() => byDay.value.get(today.value) ?? [])
const eventsTomorrow = computed(() => byDay.value.get(tomorrow.value) ?? [])

const mealSettings = useMealsSettingsStore()
const plan = ref<MealPlanEntryView[]>([])
const slots = computed(() =>
  mealSettings.visibleSlots.flatMap((s) => (s.id ? [{ id: s.id, name: s.name }] : [])),
)
const mealsToday = computed(() => mealCards(plan.value, today.value, slots.value))
const mealsTomorrow = computed(() => mealCards(plan.value, tomorrow.value, slots.value))

// Today's tasks are the server's "now" view; tomorrow's are the open ones due tomorrow.
const tasksToday = computed<TaskView[]>(() => tasks.tabs.now ?? [])
const tasksTomorrow = computed<TaskView[]>(() =>
  tasksDueOn(tasks.tabs.all ?? [], tomorrow.value, tz.value),
)

async function loadPlan() {
  try {
    const [list] = await Promise.all([
      food.plan(today.value, tomorrow.value),
      mealSettings.loaded ? Promise.resolve() : mealSettings.load(),
    ])
    plan.value = list
  } catch {
    plan.value = []
  }
}

function loadRange() {
  void calendar.ensureRange(today.value, end.value, tz.value)
}

onMounted(() => {
  void weather.load()
  void members.load()
  void tasks.load('now')
  void tasks.load('all')
  void loadPlan()
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
      <UiTopAppBar :title="todayLabel" />
    </template>

    <AdvisoryList :advisories="weather.advisories" />
    <TodayWeather
      v-if="weather.latest"
      mode="today"
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

    <UiErrorState v-if="calendar.error" :message="calendar.error" @retry="loadRange" />
    <UiSkeleton v-else-if="loadingEvents" :lines="4" />
    <TodayDay
      v-else
      :meals="mealsToday"
      :tasks="tasksToday"
      :events="eventsToday"
      :tz="tz"
      completable
      @complete="complete"
    />

    <h1 class="page-head" data-testid="tomorrow-head">Tomorrow</h1>
    <TodayWeather
      v-if="weather.latest"
      mode="tomorrow"
      :weather="weather.latest"
      :tz="tz"
      :units="household.units"
    />
    <TodayDay :meals="mealsTomorrow" :tasks="tasksTomorrow" :events="eventsTomorrow" :tz="tz" />
  </UiPage>
</template>

<style scoped>
.page-head {
  margin: var(--ui-space-2) 0 0;
  font: var(--ui-type-headline-sm);
}
</style>
