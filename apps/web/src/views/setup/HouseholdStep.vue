<script setup lang="ts">
import { reactive, ref } from 'vue'
import { api } from '../../api/client'
import { problemCode } from '../../api/http'
import { searchCities, type GeoResult } from '../../lib/geocode'
import { useSetupStore } from '../../stores/setup'
import { UiButton, UiCard, UiSelect, UiTextField } from '../../ui'
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
const unitOptions = [
  { value: 'metric', label: 'Metric (°C, km/h, mm)' },
  { value: 'imperial', label: 'Imperial (°F, mph, in)' },
]

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
  <form class="step" data-testid="step-household" @submit.prevent="next">
    <UiCard>
      <template #header>Household</template>
      <div class="stack">
        <UiTextField
          v-model="f.name"
          label="Household name"
          name="name"
          :error="errors.name"
          required
        />
        <UiTextField
          v-model="f.tz"
          label="Time zone"
          name="tz"
          list="setup-tz"
          :error="errors.tz"
          required
        />
        <datalist id="setup-tz">
          <option v-for="z in zones" :key="z" :value="z" />
        </datalist>
      </div>
    </UiCard>

    <UiCard>
      <template #header>Location (for weather)</template>
      <div class="stack">
        <div class="search">
          <UiTextField
            v-model="city"
            label="Search for a city"
            name="city"
            @keydown.enter.prevent="search"
          />
          <UiButton variant="tonal" icon="search" :loading="pending.searching" @click="search"
            >Search</UiButton
          >
        </div>
        <p v-if="searchNote" class="muted" role="status">{{ searchNote }}</p>
        <ul v-if="hits.length" class="hits" data-testid="city-hits">
          <li v-for="h in hits" :key="`${h.lat},${h.lon}`">
            <UiButton variant="text" @click="pick(h)">{{ h.label }}</UiButton>
          </li>
        </ul>
        <UiTextField v-model="f.label" label="Label" name="label" supporting="For example Home" />
        <UiTextField
          v-model="f.lat"
          label="Latitude"
          name="lat"
          inputmode="decimal"
          :error="errors.lat"
        />
        <UiTextField
          v-model="f.lon"
          label="Longitude"
          name="lon"
          inputmode="decimal"
          :error="errors.lon"
        />
      </div>
    </UiCard>

    <UiCard>
      <template #header>Units and currency</template>
      <div class="stack">
        <UiSelect v-model="f.units" label="Units" :options="unitOptions" />
        <UiTextField
          v-model="f.currency"
          label="Currency (ISO code)"
          name="currency"
          maxlength="3"
          :error="errors.currency"
          required
        />
      </div>
    </UiCard>

    <p v-if="error" class="err" role="alert">{{ error }}</p>
    <StepButtons :busy="busy" @back="setup.goto(1)" />
  </form>
</template>

<style scoped>
.step {
  display: grid;
  gap: var(--ui-space-4);
}
.stack {
  display: grid;
  gap: var(--ui-space-3);
}
.search {
  display: flex;
  align-items: end;
  gap: var(--ui-space-2);
}
.search :first-child {
  flex: 1;
}
.hits {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-1);
}
.muted {
  margin: 0;
  color: var(--ui-on-surface-variant);
}
.err {
  margin: 0;
  color: var(--ui-error);
}
</style>
