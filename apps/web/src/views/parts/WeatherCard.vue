<script setup lang="ts">
import { computed } from 'vue'
import type { WeatherView } from '../../api/client'
import WeatherIcon from '../../components/WeatherIcon.vue'
import { useUnits } from '../../composables/useUnits'
import { dayKey, formatDayHeading } from '../../lib/time'
import type { UnitSystem } from '../../lib/types'
import { weatherInfo } from '../../lib/weatherCodes'
import { columns, num, row } from '../../stores/weather'

const props = defineProps<{ weather: WeatherView; tz: string; units: UnitSystem }>()
const u = useUnits(computed(() => props.units))

const cur = computed(() => columns(props.weather.current))
const code = computed(() => num(cur.value.weather_code) ?? 0)
const isNight = computed(() => num(cur.value.is_day) === 0)
const label = computed(() => weatherInfo(code.value, isNight.value).label)
const days = computed(() => {
  const d = columns(props.weather.daily)
  const times = d.time ?? []
  const today = dayKey(new Date(), props.tz)
  return times.map((t, i) => {
    const r = row(d, i)
    return {
      key: t,
      name: formatDayHeading(t, props.tz, today).split(',')[0]!,
      code: Number(r.weather_code ?? 0),
      hi: num(r.temperature_2m_max),
      lo: num(r.temperature_2m_min),
      rain: num(r.precipitation_probability_max),
    }
  })
})
const temp = (v: number | undefined) => (v === undefined ? '–' : u.format(v, 'temp'))
</script>

<template>
  <section class="weather-card" aria-label="Weather">
    <div class="now">
      <WeatherIcon :code="code" :night="isNight" size="3rem" />
      <div>
        <div class="temp">{{ temp(num(cur.temperature_2m)) }}</div>
        <div class="muted">
          {{ label }}
          <template v-if="num(cur.apparent_temperature) !== undefined">
            · feels {{ temp(num(cur.apparent_temperature)) }}
          </template>
        </div>
      </div>
    </div>
    <ol class="strip">
      <li v-for="d in days" :key="d.key">
        <span class="muted">{{ d.name }}</span>
        <WeatherIcon :code="d.code" />
        <span>{{ temp(d.hi) }}</span>
        <span class="muted">{{ temp(d.lo) }}</span>
        <span v-if="d.rain !== undefined" class="muted">{{ Math.round(d.rain) }}%</span>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.weather-card {
  padding: var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}
.now {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
.temp {
  font-size: var(--font-size-lg);
  font-weight: 600;
}
.muted {
  color: var(--color-muted);
  font-size: var(--font-size-sm);
}
.strip {
  display: flex;
  gap: var(--space-3);
  list-style: none;
  padding: 0;
  margin: var(--space-3) 0 0;
  overflow-x: auto;
}
.strip li {
  display: grid;
  justify-items: center;
  gap: var(--space-1);
  min-width: 4rem;
}
</style>
