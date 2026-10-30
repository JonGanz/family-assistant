<script setup lang="ts">
import { computed } from 'vue'
import WeatherIcon from '../../components/WeatherIcon.vue'
import { useUnits } from '../../composables/useUnits'
import { dayKey } from '../../lib/time'
import { weatherInfo } from '../../lib/weatherCodes'
import Modal from '../Modal.vue'
import { S } from '../strings'
import type { DashboardData } from '../types'
import { dailyOf, hourlyOf } from '../weatherModel'

const props = defineProps<{ data: DashboardData; now: Date }>()
defineEmits<{ close: [] }>()
const { format } = useUnits(computed(() => props.data.household.units))
const w = computed(() => props.data.weather ?? null)
const today = computed(() => dayKey(props.now, props.data.household.tz))
const hours = computed(() =>
  w.value ? hourlyOf(w.value, 48).filter((h) => h.date === today.value) : [],
)
const days = computed(() => (w.value ? dailyOf(w.value, 16) : []))
const t = (v: number | null) => (v === null ? '–' : format(v, 'temp'))
const weekday = (d: string) =>
  d === today.value
    ? S.today
    : new Intl.DateTimeFormat(undefined, {
        weekday: 'short',
        month: 'short',
        day: 'numeric',
        timeZone: 'UTC',
      }).format(new Date(`${d}T12:00:00Z`))
</script>

<template>
  <Modal :title="S.forecastTitle" @close="$emit('close')">
    <h3>{{ S.hourlyToday }}</h3>
    <ul class="hours" aria-label="Hourly forecast">
      <li v-for="h in hours" :key="h.label">
        <span class="muted">{{ h.label }}</span>
        <WeatherIcon :code="h.code" size="2.4rem" />
        <span class="hi">{{ t(h.temp) }}</span>
        <span v-if="h.pop !== null" class="muted">{{ Math.round(h.pop) }}%</span>
      </li>
    </ul>
    <h3>{{ S.upcoming }}</h3>
    <ul class="days" aria-label="Daily forecast">
      <li v-for="d in days" :key="d.date">
        <span class="d">{{ weekday(d.date) }}</span>
        <WeatherIcon :code="d.code" size="2.4rem" />
        <span class="muted label">{{ weatherInfo(d.code, false).label }}</span>
        <span class="hi">{{ t(d.max) }}</span>
        <span class="muted">{{ t(d.min) }}</span>
        <span class="muted">{{ d.pop === null ? '' : `${Math.round(d.pop)}%` }}</span>
      </li>
    </ul>
  </Modal>
</template>

<style scoped>
h3 {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-base);
  color: var(--color-muted);
}
.muted {
  color: var(--color-muted);
}
.hi {
  font-weight: 700;
}
ul {
  list-style: none;
  margin: 0 0 var(--space-4);
  padding: 0;
}
.hours {
  display: flex;
  gap: var(--space-2);
  overflow-x: auto;
  padding-bottom: var(--space-2);
}
.hours li {
  flex: none;
  display: grid;
  justify-items: center;
  gap: var(--space-1);
  min-width: 4.5rem;
  padding: var(--space-2);
  background: var(--color-surface);
}
.days li {
  display: grid;
  grid-template-columns: 8em 3rem 1fr 4em 4em 4em;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) 0;
  border-bottom: 1px solid var(--color-border);
}
.label {
  min-width: 0;
}
</style>
