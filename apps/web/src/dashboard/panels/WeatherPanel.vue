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
/** Before noon (household time) the day's forecast is shown beside the current temperature. */
const morning = computed(() => {
  const h = new Intl.DateTimeFormat('en-GB', {
    timeZone: props.data.household.tz,
    hourCycle: 'h23',
    hour: '2-digit',
  }).format(props.now)
  return Number(h) < 12
})
const todayFc = computed(() => (morning.value && w.value ? dailyOf(w.value, 1)[0] : undefined))
const t = (v: number | null) => (v === null ? '–' : format(v, 'temp'))
const weekday = (d: string) =>
  new Intl.DateTimeFormat(undefined, { weekday: 'short', timeZone: 'UTC' }).format(
    new Date(`${d}T12:00:00Z`),
  )
const THUNDER = new Set([95, 96, 99])
/** Upcoming days (today included) whose forecast code is a thunderstorm. */
const thunderDays = computed(() => days.value.filter((d) => THUNDER.has(d.code)))
const thunderText = computed(() => {
  const ds = thunderDays.value
  if (!ds.length) return ''
  const first = ds[0]!
  const name = first.date === days.value[0]?.date ? S.today : weekday(first.date)
  return `${S.thunderForecast}: ${name}${ds.length > 1 ? ` +${ds.length - 1}` : ''}`
})
const notices = computed(() => [
  ...props.data.advisories.map((a) => ({ id: a.id, text: a.text, severity: a.severity as string })),
  ...(thunderText.value ? [{ id: 'thunder', text: thunderText.value, severity: 'warning' }] : []),
])
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
        <div v-if="todayFc" class="today" data-testid="today-forecast">
          <div class="head">{{ S.today }}</div>
          <div class="range">
            <span class="hi">{{ t(todayFc.max) }}</span>
            <span class="lo">{{ t(todayFc.min) }}</span>
          </div>
          <div v-if="todayFc.pop !== null" class="rain">
            {{ S.rainChance }} {{ Math.round(todayFc.pop) }}%
          </div>
        </div>
        <ul v-if="notices.length" class="notices" aria-label="Advisories">
          <li
            v-for="n in notices"
            :key="n.id"
            :class="n.severity"
            :data-severity="n.severity"
            role="status"
          >
            {{ n.text }}
          </li>
        </ul>
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
.today {
  display: grid;
  gap: var(--space-1);
  padding-left: var(--space-3);
  border-left: 1px solid var(--color-border);
}
.today .head,
.today .rain {
  font-size: var(--font-size-base);
  color: var(--color-muted);
}
.today .range {
  font-size: 1.6em;
  font-weight: 600;
  display: flex;
  gap: var(--space-2);
}
.notices {
  margin-left: auto;
  display: grid;
  gap: var(--space-1);
  align-content: center;
  min-width: 0;
  --sev: var(--color-accent);
}
.notices li {
  padding: var(--space-1) var(--space-2);
  border-left: 6px solid var(--sev);
  background: var(--color-surface);
}
.notices li.warning {
  --sev: var(--color-warning);
}
.notices li.severe {
  --sev: var(--color-danger);
  font-weight: 600;
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
