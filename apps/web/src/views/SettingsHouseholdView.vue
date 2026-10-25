<script setup lang="ts">
import { onMounted, reactive, ref, watch } from 'vue'
import ErrorBanner from '../components/ErrorBanner.vue'
import { useHouseholdStore } from '../stores/household'
import { describeError } from './shared'
import './parts/settings.css'

const household = useHouseholdStore()
const form = reactive({
  name: '',
  tz: '',
  label: '',
  lat: '',
  lon: '',
  units: 'metric' as 'metric' | 'imperial',
  past: 30,
  future: 400,
  retention: 7,
  asBusy: true,
  override: true,
  nowSize: 7,
  wDue: 1,
  wFlag: 0.5,
  wWeather: 0.5,
  quietFrom: '21:00',
  quietUntil: '07:00',
})
const message = ref('')
const error = ref('')
const geoNote = ref('')

const zones: string[] =
  typeof Intl.supportedValuesOf === 'function' ? Intl.supportedValuesOf('timeZone') : []

function fill() {
  const v = household.view
  if (!v) return
  form.name = v.name
  form.tz = v.tz
  form.label = v.location?.label ?? ''
  form.lat = v.location ? String(v.location.lat) : ''
  form.lon = v.location ? String(v.location.lon) : ''
  form.units = v.units
  form.past = v.calendar_past_days
  form.future = v.calendar_future_days
  form.retention = v.weather_retention_days
  form.asBusy = v.private_events_as_busy
  form.override = v.dashboard_completion_override
  form.nowSize = v.now_list_size
  form.wDue = v.now_weights.due
  form.wFlag = v.now_weights.flag
  form.wWeather = v.now_weights.weather
  form.quietFrom = v.quiet_from
  form.quietUntil = v.quiet_until
}

onMounted(async () => {
  await household.load()
  fill()
})
watch(() => household.view?.version, fill)

function useDeviceLocation() {
  geoNote.value = ''
  if (!('geolocation' in navigator)) {
    geoNote.value = 'This browser cannot share its location. Enter the coordinates by hand.'
    return
  }
  geoNote.value = 'Asking for your location…'
  navigator.geolocation.getCurrentPosition(
    (pos) => {
      form.lat = pos.coords.latitude.toFixed(4)
      form.lon = pos.coords.longitude.toFixed(4)
      if (!form.label) form.label = 'Home'
      geoNote.value = 'Location filled in. Save to keep it.'
    },
    (err) => {
      geoNote.value =
        err.code === err.PERMISSION_DENIED
          ? 'Location access was denied. Enter the coordinates by hand.'
          : 'Could not get the location. Enter the coordinates by hand.'
    },
    { timeout: 10_000 },
  )
}

async function save() {
  message.value = ''
  error.value = ''
  const hasLoc = form.lat.trim() !== '' && form.lon.trim() !== ''
  const lat = Number(form.lat)
  const lon = Number(form.lon)
  if (
    hasLoc &&
    (!Number.isFinite(lat) || !Number.isFinite(lon) || Math.abs(lat) > 90 || Math.abs(lon) > 180)
  ) {
    error.value = 'Latitude must be between -90 and 90, and longitude between -180 and 180.'
    return
  }
  try {
    const ok = await household.save({
      name: form.name,
      tz: form.tz,
      units: form.units,
      calendar_past_days: form.past,
      calendar_future_days: form.future,
      weather_retention_days: form.retention,
      private_events_as_busy: form.asBusy,
      dashboard_completion_override: form.override,
      now_list_size: form.nowSize,
      now_weights: { due: form.wDue, flag: form.wFlag, weather: form.wWeather },
      quiet_from: form.quietFrom,
      quiet_until: form.quietUntil,
      location: hasLoc ? { label: form.label || 'Home', lat, lon } : null,
    })
    if (ok) message.value = 'Saved.'
  } catch (e) {
    error.value = describeError(e, 'Save failed.')
  }
}

async function reload() {
  await household.load()
  fill()
}
</script>

<template>
  <section>
    <h1>Household</h1>
    <form class="settings-form" @submit.prevent="save">
      <label>Name <input v-model="form.name" name="name" required /></label>
      <label>
        Time zone
        <input v-model="form.tz" name="tz" list="tz-list" required />
        <datalist id="tz-list">
          <option v-for="z in zones" :key="z" :value="z" />
        </datalist>
      </label>
      <fieldset>
        <legend>Location (for weather)</legend>
        <label>Label <input v-model="form.label" name="label" placeholder="Home" /></label>
        <label>Latitude <input v-model="form.lat" name="lat" inputmode="decimal" /></label>
        <label>Longitude <input v-model="form.lon" name="lon" inputmode="decimal" /></label>
        <button type="button" @click="useDeviceLocation">Use this device's location</button>
        <p v-if="geoNote" class="settings-note" role="status">{{ geoNote }}</p>
      </fieldset>
      <label>
        Units
        <select v-model="form.units" name="units">
          <option value="metric">Metric (°C, km/h, mm)</option>
          <option value="imperial">Imperial (°F, mph, in)</option>
        </select>
      </label>
      <fieldset>
        <legend>Calendar window</legend>
        <label>
          Days of past events to keep
          <input v-model.number="form.past" name="past" type="number" min="0" max="3650" />
        </label>
        <label>
          Days ahead to sync
          <input v-model.number="form.future" name="future" type="number" min="1" max="3650" />
        </label>
      </fieldset>
      <label>
        Days of weather history to keep
        <input v-model.number="form.retention" name="retention" type="number" min="1" max="365" />
      </label>
      <fieldset>
        <legend>Tasks</legend>
        <label class="inline">
          <input v-model="form.override" name="dashboard_completion_override" type="checkbox" />
          Allow ticking off tasks on dashboards
        </label>
        <label>
          How many tasks the "Now" list shows
          <input
            v-model.number="form.nowSize"
            name="now_list_size"
            type="number"
            min="1"
            max="50"
          />
        </label>
        <p class="settings-note">
          What floats to the top of "Now": higher numbers count for more. Due date is how close or
          late a task is, priority is the high-priority flag, weather is a task made by a weather
          rule.
        </p>
        <label
          >Weight: due date
          <input v-model.number="form.wDue" name="w_due" type="number" min="0" step="0.1"
        /></label>
        <label
          >Weight: priority
          <input v-model.number="form.wFlag" name="w_flag" type="number" min="0" step="0.1"
        /></label>
        <label
          >Weight: weather
          <input v-model.number="form.wWeather" name="w_weather" type="number" min="0" step="0.1"
        /></label>
      </fieldset>
      <fieldset>
        <legend>Default quiet hours (no notifications)</legend>
        <label
          >From <input v-model="form.quietFrom" name="quiet_from" type="time" required
        /></label>
        <label
          >Until <input v-model="form.quietUntil" name="quiet_until" type="time" required
        /></label>
      </fieldset>
      <label class="inline">
        <input v-model="form.asBusy" name="private_events_as_busy" type="checkbox" />
        Show adults' private events as "Busy" to children and on the dashboard
      </label>
      <ErrorBanner v-if="household.conflict" conflict @reload="reload" />
      <ErrorBanner v-if="error" :message="error" />
      <p v-if="message" class="settings-ok" role="status">{{ message }}</p>
      <button type="submit">Save</button>
    </form>
  </section>
</template>
