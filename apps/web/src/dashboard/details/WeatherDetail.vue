<script setup lang="ts">
import { computed } from 'vue'
import WeatherIcon from '../../components/WeatherIcon.vue'
import { useUnits } from '../../composables/useUnits'
import { dayKey } from '../../lib/time'
import { weatherInfo } from '../../lib/weatherCodes'
import Modal from '../Modal.vue'
import { S } from '../strings'
import type { DashboardData } from '../types'
import { dailyOf, hourlyOf, localHourKey, nextHours } from '../weatherModel'

const props = defineProps<{ data: DashboardData; now: Date }>()
defineEmits<{ close: [] }>()
const { format } = useUnits(computed(() => props.data.household.units))
const w = computed(() => props.data.weather ?? null)
const today = computed(() => dayKey(props.now, props.data.household.tz))
const hours = computed(() =>
  w.value
    ? nextHours(hourlyOf(w.value, 96), localHourKey(props.now, props.data.household.tz), 24)
    : [],
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
    <h3>{{ S.hourlyNext24 }}</h3>
    <ul class="hours" aria-label="Hourly forecast">
      <li v-for="(h, i) in hours" :key="h.date + h.label">
        <span v-if="h.date !== today && (i === 0 || hours[i - 1]!.date !== h.date)" class="day">
          {{ weekday(h.date) }}
        </span>
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
        <span class="hi">{{ t(d.max) }}</span>
        <span class="muted">{{ t(d.min) }}</span>
        <WeatherIcon :code="d.code" size="2.4rem" />
        <span class="muted label">{{ weatherInfo(d.code, false).label }}</span>
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
.day {
  font-size: var(--font-size-sm, 0.8em);
  font-weight: 700;
}
.hi {
  font-weight: 700;
}
ul {
  list-style: none;
  margin: 0 0 var(--space-4);
  padding: 0;
}
/* The hourly strip scrolls inside the modal; it must not set the modal's width. */
.hours {
  contain: inline-size;
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
  border-radius: var(--ui-radius-lg);
  background: var(--color-layer-3);
}
/* One shared grid so each column is as wide as its widest cell and no wider. */
.days {
  display: grid;
  grid-template-columns: repeat(6, max-content);
  column-gap: var(--space-4);
  row-gap: var(--space-1);
  width: fit-content;
}
.days li {
  display: grid;
  grid-column: 1 / -1;
  grid-template-columns: subgrid;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--ui-radius-md);
  background: var(--color-layer-3);
}
.label {
  white-space: nowrap;
}
</style>
