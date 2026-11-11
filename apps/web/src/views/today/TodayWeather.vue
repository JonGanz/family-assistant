<script setup lang="ts">
import { computed, ref } from 'vue'
import type { WeatherView } from '../../api/client'
import WeatherIcon from '../../components/WeatherIcon.vue'
import { useUnits } from '../../composables/useUnits'
import { addDays, dayKey, formatDayHeading } from '../../lib/time'
import type { UnitSystem } from '../../lib/types'
import { weatherInfo } from '../../lib/weatherCodes'
import { columns, num } from '../../stores/weather'
import { UiCard, UiDialog } from '../../ui'
import { dailyRows, hoursOf } from './todayModel'

const props = defineProps<{
  weather: WeatherView
  tz: string
  units: UnitSystem
  /** `today`: current conditions, tap for the hours. `tomorrow`: the remaining days (no dialog). */
  mode: 'today' | 'tomorrow'
}>()
const u = useUnits(computed(() => props.units))
const open = ref(false)

const today = computed(() => dayKey(new Date(), props.tz))
const tomorrow = computed(() => addDays(today.value, 1))
const cur = computed(() => columns(props.weather.current))
const code = computed(() => num(cur.value.weather_code) ?? 0)
const isNight = computed(() => num(cur.value.is_day) === 0)
const days = computed(() => dailyRows(props.weather.daily))
const day = computed(() => days.value.find((d) => d.date === today.value))
const rest = computed(() => days.value.filter((d) => d.date >= tomorrow.value))
const hours = computed(() => hoursOf(props.weather.hourly, today.value))
/** Only today's card opens the hours; tomorrow's card already shows the daily forecast. */
const show = () => {
  if (props.mode === 'today') open.value = true
}
const temp = (v: number | undefined) => (v === undefined ? '–' : u.format(v, 'temp'))
const label = (c: number, night = false) => weatherInfo(c, night).label
const dayName = (d: string) => formatDayHeading(d, props.tz, today.value).split(',')[0]!
const title = computed(() => (props.mode === 'today' ? 'Today’s weather' : 'Tomorrow’s weather'))
</script>

<template>
  <UiCard
    variant="filled"
    class="weather"
    :class="{ tappable: mode === 'today' }"
    :role="mode === 'today' ? 'button' : undefined"
    :tabindex="mode === 'today' ? 0 : undefined"
    :aria-label="mode === 'today' ? `${title}: show the hours` : title"
    data-testid="today-weather"
    @click="show"
    @keydown.enter="show"
    @keydown.space.prevent="show"
  >
    <template v-if="mode === 'today'">
      <div class="now">
        <WeatherIcon :code="code" :night="isNight" size="3rem" />
        <div>
          <div class="temp">{{ temp(num(cur.temperature_2m)) }}</div>
          <div class="muted">
            {{ label(code, isNight) }}
            <template v-if="num(cur.apparent_temperature) !== undefined">
              · feels {{ temp(num(cur.apparent_temperature)) }}
            </template>
          </div>
          <div v-if="day" class="muted">
            High {{ temp(day.hi) }} · Low {{ temp(day.lo)
            }}<template v-if="day.rain !== undefined"> · {{ Math.round(day.rain) }}% rain</template>
          </div>
        </div>
      </div>
    </template>
    <ol v-else class="strip" aria-label="Forecast">
      <li v-for="d in rest" :key="d.date" data-testid="forecast-day">
        <span class="muted">{{ dayName(d.date) }}</span>
        <WeatherIcon :code="d.code" />
        <span>{{ temp(d.hi) }}</span>
        <span class="muted">{{ temp(d.lo) }}</span>
        <span v-if="d.rain !== undefined" class="muted">{{ Math.round(d.rain) }}%</span>
      </li>
    </ol>
  </UiCard>

  <UiDialog v-if="mode === 'today'" v-model="open" :title="title">
    <p v-if="!hours.length" class="muted">No hourly data.</p>
    <ol v-else class="strip" aria-label="Hourly">
      <li v-for="h in hours" :key="h.time" data-testid="hour-row">
        <span class="muted">{{ h.time }}</span>
        <WeatherIcon :code="h.code" />
        <span>{{ temp(h.temp) }}</span>
        <span v-if="h.rain !== undefined" class="muted">{{ Math.round(h.rain) }}%</span>
      </li>
    </ol>
  </UiDialog>
</template>

<style scoped>
.tappable {
  cursor: pointer;
}
.now {
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
}
.temp {
  font: var(--ui-type-headline-sm);
}
.muted {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
  margin: 0;
}
.strip {
  display: flex;
  gap: var(--ui-space-2);
  list-style: none;
  padding: 0 0 var(--ui-space-1);
  margin: 0;
  overflow-x: auto;
}
.strip li {
  display: grid;
  justify-items: center;
  gap: var(--ui-space-1);
  min-width: 4.5rem;
  padding: var(--ui-space-2);
  border-radius: var(--ui-radius-md);
  background: var(--ui-surface-container-high);
  --ui-on-surface-variant: var(--ui-on-surface-variant-high);
  font: var(--ui-type-body-md);
}
</style>
