<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { addDays, dayKey } from '../lib/time'
import { useCalendarStore } from '../stores/calendar'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import { useWeatherStore } from '../stores/weather'
import AdvisoryList from './parts/AdvisoryList.vue'
import AgendaList from './parts/AgendaList.vue'
import WeatherCard from './parts/WeatherCard.vue'
import { memberLikes } from './shared'

const session = useSessionStore()
const household = useHouseholdStore()
const calendar = useCalendarStore()
const weather = useWeatherStore()
const members = useMembersStore()

const tz = computed(() => household.tz)
const today = computed(() => dayKey(new Date(), tz.value))
const end = computed(() => addDays(today.value, 2))
const events = computed(() => calendar.eventsIn(today.value, end.value, tz.value))
const likes = computed(() => memberLikes(members.items))

function loadRange() {
  void calendar.ensureRange(today.value, end.value, tz.value)
}

onMounted(() => {
  void weather.load()
  void members.load()
  loadRange()
})
watch(tz, loadRange)
</script>

<template>
  <section>
    <h1>{{ session.household?.name ?? 'Today' }}</h1>
    <AdvisoryList :advisories="weather.advisories" @dismiss="weather.dismiss" />
    <WeatherCard
      v-if="weather.latest"
      :weather="weather.latest"
      :tz="tz"
      :units="household.units"
    />
    <p v-else-if="weather.loaded" class="muted">
      No weather yet.
      <RouterLink v-if="session.isAdult" to="/settings/household"
        >Set the household location</RouterLink
      >
      to enable it.
    </p>
    <p v-if="calendar.error" class="error" role="alert">{{ calendar.error }}</p>
    <AgendaList :events="events" :tz="tz" :from="today" :days="2" :members="likes" />
  </section>
</template>

<style scoped>
.muted {
  color: var(--color-muted);
}
.error {
  color: var(--color-danger);
}
</style>
