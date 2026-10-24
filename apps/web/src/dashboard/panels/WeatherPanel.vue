<script setup lang="ts">
import { computed } from 'vue'
import WeatherIcon from '../../components/WeatherIcon.vue'
import { useUnits } from '../../composables/useUnits'
import { weatherInfo } from '../../lib/weatherCodes'
import { formatQuantity } from '../../lib/units'
import { S } from '../strings'
import type { PanelProps } from '../types'
import { currentOf, dailyOf, hourlyOf } from '../weatherModel'

const props = defineProps<PanelProps>()
const units = computed(() => props.data.household.units)
const { format } = useUnits(units)
const w = computed(() => props.data.weather ?? null)
const cur = computed(() => (w.value ? currentOf(w.value) : null))
const showHourly = computed(() => props.panel['hourly'] === true)
const hours = computed(() => (w.value && showHourly.value ? hourlyOf(w.value, 12) : []))
const days = computed(() => (w.value ? dailyOf(w.value, showHourly.value ? 5 : 7) : []))
const t = (v: number | null) => (v === null ? '–' : format(v, 'temp'))
const weekday = (d: string) =>
  new Intl.DateTimeFormat(undefined, { weekday: 'short', timeZone: 'UTC' }).format(
    new Date(`${d}T12:00:00Z`),
  )
const hour = (label: string) => (label ? label.slice(0, 2) : '')
</script>

<template>
  <section class="weather" aria-label="Weather">
    <p v-if="!w || !cur" class="empty">{{ S.noWeather }}</p>
    <template v-else>
      <div class="now">
        <WeatherIcon :code="cur.code" :night="cur.night" size="2.6em" />
        <div>
          <div class="temp" data-testid="temp">{{ t(cur.temp) }}</div>
          <div class="label">{{ weatherInfo(cur.code, cur.night).label }}</div>
        </div>
        <div class="extra">
          <div v-if="cur.feels !== null">{{ S.feelsLike }} {{ t(cur.feels) }}</div>
          <div v-if="cur.wind !== null">
            {{ S.wind }} {{ formatQuantity(cur.wind, 'speed', units) }}
          </div>
        </div>
      </div>
      <ul v-if="hours.length" class="hours" aria-label="Next hours">
        <li v-for="(h, i) in hours" :key="i">
          <span class="h">{{ hour(h.label) }}</span>
          <WeatherIcon :code="h.code" size="1.2em" />
          <span>{{ t(h.temp) }}</span>
        </li>
      </ul>
      <ul class="days" aria-label="Forecast">
        <li v-for="d in days" :key="d.date">
          <span class="d">{{ weekday(d.date) }}</span>
          <WeatherIcon :code="d.code" size="1.2em" />
          <span class="hi">{{ t(d.max) }}</span>
          <span class="lo">{{ t(d.min) }}</span>
        </li>
      </ul>
    </template>
  </section>
</template>

<style scoped>
.weather {
  display: grid;
  gap: var(--space-3);
}
.empty {
  color: var(--color-muted);
  margin: 0;
}
.now {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
.temp {
  font-size: 2.2em;
  font-weight: 600;
  line-height: 1.1;
}
.label,
.extra {
  font-size: var(--font-size-base);
  color: var(--color-muted);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  font-size: var(--font-size-base);
}
.hours {
  display: flex;
  gap: var(--space-2);
  overflow: hidden;
}
.hours li {
  display: grid;
  justify-items: center;
  min-width: 2.2em;
}
.h {
  color: var(--color-muted);
}
.days li {
  display: grid;
  grid-template-columns: 3em 2em 1fr 1fr;
  align-items: center;
  gap: var(--space-2);
}
.lo {
  color: var(--color-muted);
}
</style>
