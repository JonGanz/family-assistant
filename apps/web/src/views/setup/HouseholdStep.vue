<script setup lang="ts">
import { reactive, ref } from 'vue'
import { api } from '../../api/client'
import { problemCode } from '../../api/http'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { searchCities, type GeoResult } from '../../lib/geocode'
import { useSetupStore } from '../../stores/setup'
import { describeError } from '../shared'
import StepButtons from './StepButtons.vue'
import { validateHousehold } from './validate'

const setup = useSetupStore()
const f = setup.household
const errors = ref<Record<string, string>>({})
const error = ref('')
const busy = ref(false)
const city = ref('')
const hits = ref<GeoResult[]>([])
const searchNote = ref('')
const zones: string[] =
  typeof Intl.supportedValuesOf === 'function' ? Intl.supportedValuesOf('timeZone') : []
const pending = reactive({ searching: false })

async function search() {
  searchNote.value = ''
  hits.value = []
  pending.searching = true
  try {
    hits.value = await searchCities(city.value)
    if (!hits.value.length) searchNote.value = 'No match. Enter the coordinates by hand below.'
  } catch {
    searchNote.value = 'The city search is not reachable. Enter the coordinates by hand below.'
  } finally {
    pending.searching = false
  }
}

function pick(h: GeoResult) {
  f.label = h.label
  f.lat = h.lat.toFixed(4)
  f.lon = h.lon.toFixed(4)
  if (h.tz) f.tz = h.tz
  hits.value = []
}

async function next() {
  error.value = ''
  errors.value = validateHousehold(f)
  if (Object.keys(errors.value).length) return
  if (!setup.status?.needed) {
    setup.goto(setup.step + 1)
    return
  }
  busy.value = true
  try {
    const hasLoc = f.lat.trim() !== ''
    await api.setupHousehold({
      name: f.name.trim(),
      tz: f.tz.trim(),
      location: hasLoc
        ? { label: f.label.trim() || 'Home', lat: Number(f.lat), lon: Number(f.lon) }
        : null,
      units: f.units,
      currency: f.currency.trim().toUpperCase(),
      base_url: location.origin,
    })
    await setup.load(true)
    setup.goto(3)
  } catch (e) {
    if (problemCode(e) === 'setup_closed') await setup.load(true)
    error.value = describeError(e, 'Could not save the household.')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <form class="form" data-testid="step-household" @submit.prevent="next">
    <h2>Household</h2>
    <label class="field">
      Household name
      <input v-model="f.name" name="name" required />
    </label>
    <p v-if="errors.name" class="text-danger" role="alert">{{ errors.name }}</p>
    <label class="field">
      Time zone
      <input v-model="f.tz" name="tz" list="setup-tz" required />
      <datalist id="setup-tz">
        <option v-for="z in zones" :key="z" :value="z" />
      </datalist>
    </label>
    <p v-if="errors.tz" class="text-danger" role="alert">{{ errors.tz }}</p>
    <fieldset>
      <legend>Location (for weather)</legend>
      <div class="row">
        <label class="field grow">
          Search for a city
          <input v-model="city" name="city" @keydown.enter.prevent="search" />
        </label>
        <button type="button" :disabled="pending.searching" @click="search">Search</button>
      </div>
      <p v-if="searchNote" class="muted" role="status">{{ searchNote }}</p>
      <ul v-if="hits.length" class="card-list" data-testid="city-hits">
        <li v-for="h in hits" :key="`${h.lat},${h.lon}`">
          <button type="button" class="btn-ghost" @click="pick(h)">{{ h.label }}</button>
        </li>
      </ul>
      <label class="field">Label <input v-model="f.label" name="label" placeholder="Home" /></label>
      <label class="field">Latitude <input v-model="f.lat" name="lat" inputmode="decimal" /></label>
      <label class="field"
        >Longitude <input v-model="f.lon" name="lon" inputmode="decimal"
      /></label>
      <p v-if="errors.lat" class="text-danger" role="alert">{{ errors.lat }}</p>
      <p v-if="errors.lon" class="text-danger" role="alert">{{ errors.lon }}</p>
    </fieldset>
    <label class="field">
      Units
      <select v-model="f.units" name="units">
        <option value="metric">Metric (°C, km/h, mm)</option>
        <option value="imperial">Imperial (°F, mph, in)</option>
      </select>
    </label>
    <label class="field">
      Currency (ISO code)
      <input v-model="f.currency" name="currency" maxlength="3" required />
    </label>
    <p v-if="errors.currency" class="text-danger" role="alert">{{ errors.currency }}</p>
    <ErrorBanner v-if="error" :message="error" />
    <StepButtons :busy="busy" @back="setup.goto(1)" />
  </form>
</template>
