<script setup lang="ts">
// Reference page (M4). One UiCard per group. Switches and single choices apply at once with Undo
// (useInstantSave); every multi-field group lives in ONE draft with one Save bar, because the API
// takes a single versioned PATCH (per-card drafts would race on the version).
import { computed, onMounted, ref, watch, type Ref } from 'vue'
import { ApiError } from '../api/http'
import type { ContentPolicy } from '../api/mailSettings'
import SettingsPage from '../app/SettingsPage.vue'
import { useHouseholdStore } from '../stores/household'
import {
  UiButton,
  UiCard,
  UiSaveBar,
  UiSegmented,
  UiSelect,
  UiSwitch,
  UiTextField,
  useFormDraft,
  useInstantSave,
} from '../ui'

const household = useHouseholdStore()

interface Draft {
  name: string
  tz: string
  label: string
  lat: string
  lon: string
  currency: string
  past: string
  future: string
  retention: string
  nowSize: string
  wDue: string
  wFlag: string
  wWeather: string
  quietFrom: string
  quietUntil: string
  extractThreshold: string
  routeThreshold: string
  taskExpiry: string
  contactExpiry: string
  female: string
  male: string
  billLead: string
  policy: ContentPolicy
  examples: string
  maxPending: string
}

function fromView(): Draft {
  const v = household.view
  return {
    name: v?.name ?? '',
    tz: v?.tz ?? '',
    label: v?.location?.label ?? '',
    lat: v?.location ? String(v.location.lat) : '',
    lon: v?.location ? String(v.location.lon) : '',
    currency: v?.currency ?? 'USD',
    past: String(v?.calendar_past_days ?? 30),
    future: String(v?.calendar_future_days ?? 400),
    retention: String(v?.weather_retention_days ?? 7),
    nowSize: String(v?.now_list_size ?? 7),
    wDue: String(v?.now_weights.due ?? 1),
    wFlag: String(v?.now_weights.flag ?? 0.5),
    wWeather: String(v?.now_weights.weather ?? 0.5),
    quietFrom: v?.quiet_from ?? '21:00',
    quietUntil: v?.quiet_until ?? '07:00',
    extractThreshold: String(v?.events?.extract_threshold ?? 0.9),
    routeThreshold: String(v?.events?.route_threshold ?? 0.9),
    taskExpiry: String(v?.inbox?.task_expiry_days ?? 14),
    contactExpiry: String(v?.inbox?.contact_expiry_days ?? 30),
    female: (v?.gender_terms?.female ?? []).join(', '),
    male: (v?.gender_terms?.male ?? []).join(', '),
    billLead: String(v?.bill_lead_days ?? 3),
    policy: v?.retention_mail?.content_policy ?? 'purge_after_processing',
    examples: String(v?.retention_mail?.examples_per_category ?? 50),
    maxPending: String(v?.retention_mail?.max_pending_days ?? 14),
  }
}

const words = (s: string) =>
  s
    .split(',')
    .map((x) => x.trim())
    .filter(Boolean)

const CHANGED = 'Changed elsewhere. Reload to see the latest, then try again.'
const fieldErrors = ref<Partial<Record<keyof Draft, string>>>({})
const geoNote = ref('')

const form = useFormDraft(fromView, async (d) => {
  fieldErrors.value = {}
  const hasLoc = d.lat.trim() !== '' && d.lon.trim() !== ''
  const lat = Number(d.lat)
  const lon = Number(d.lon)
  if (
    hasLoc &&
    (!Number.isFinite(lat) || !Number.isFinite(lon) || Math.abs(lat) > 90 || Math.abs(lon) > 180)
  ) {
    fieldErrors.value = { lat: 'Between -90 and 90', lon: 'Between -180 and 180' }
    throw new Error('Check the location.')
  }
  try {
    const ok = await household.save({
      name: d.name,
      tz: d.tz,
      currency: d.currency.trim().toUpperCase(),
      calendar_past_days: Number(d.past),
      calendar_future_days: Number(d.future),
      weather_retention_days: Number(d.retention),
      now_list_size: Number(d.nowSize),
      now_weights: { due: Number(d.wDue), flag: Number(d.wFlag), weather: Number(d.wWeather) },
      quiet_from: d.quietFrom,
      quiet_until: d.quietUntil,
      events: {
        auto_write: household.view?.events?.auto_write ?? false,
        extract_threshold: Number(d.extractThreshold),
        route_threshold: Number(d.routeThreshold),
      },
      inbox: {
        task_expiry_days: Number(d.taskExpiry),
        contact_expiry_days: Number(d.contactExpiry),
      },
      gender_terms: { female: words(d.female), male: words(d.male) },
      bill_lead_days: Number(d.billLead),
      retention_mail: {
        content_policy: d.policy,
        examples_per_category: Number(d.examples),
        max_pending_days: Number(d.maxPending),
      },
      location: hasLoc ? { label: d.label || 'Home', lat, lon } : null,
    })
    if (!ok) throw new Error(CHANGED)
  } catch (e) {
    // A problem that names its field is shown under that field.
    const field = e instanceof ApiError ? e.problem.field : undefined
    if (e instanceof ApiError && typeof field === 'string' && field in d) {
      fieldErrors.value = { [field]: e.message }
      throw new Error('Check the highlighted field.', { cause: e })
    }
    throw e
  }
})
const d = form.draft

// Instant controls: each mirrors one server field and saves alone.
function instant<T>(
  read: () => T,
  patch: (v: T) => Parameters<typeof household.save>[0],
  label: string,
) {
  const value = ref(read()) as Ref<T>
  const { set, saving } = useInstantSave(
    value,
    async (v) => {
      if (!(await household.save(patch(v)))) throw new Error(CHANGED)
    },
    { label },
  )
  watch(read, (v) => (value.value = v))
  return { value, set, saving }
}
const units = instant(
  () => household.view?.units ?? 'metric',
  (v) => ({ units: v }),
  'Units saved',
)
const weekStart = instant(
  () => household.view?.week_start ?? 'monday',
  (v) => ({ week_start: v }),
  'First day saved',
)
const asBusy = instant(
  () => household.view?.private_events_as_busy ?? true,
  (v) => ({ private_events_as_busy: v }),
  'Saved',
)
const override = instant(
  () => household.view?.dashboard_completion_override ?? true,
  (v) => ({ dashboard_completion_override: v }),
  'Saved',
)
const pendingBadge = instant(
  () => household.view?.dashboard_pending_approvals ?? true,
  (v) => ({ dashboard_pending_approvals: v }),
  'Saved',
)
const autoWrite = instant(
  () => household.view?.events?.auto_write ?? false,
  (v) => ({
    events: {
      auto_write: v,
      extract_threshold: household.view?.events?.extract_threshold ?? 0.9,
      route_threshold: household.view?.events?.route_threshold ?? 0.9,
    },
  }),
  'Saved',
)

const zones: string[] =
  typeof Intl.supportedValuesOf === 'function' ? Intl.supportedValuesOf('timeZone') : []
const zoneOptions = computed(() => {
  const all = d.value.tz && !zones.includes(d.value.tz) ? [d.value.tz, ...zones] : zones
  return all.map((z) => ({ value: z, label: z }))
})
const policyOptions = [
  { value: 'purge_after_processing', label: 'Delete after processing' },
  { value: 'keep_for_learning', label: 'Keep examples for learning' },
  { value: 'keep_all', label: 'Keep everything' },
]

onMounted(() => household.load())

async function reload() {
  form.discard()
  await household.load()
}

function useDeviceLocation() {
  geoNote.value = ''
  if (!('geolocation' in navigator)) {
    geoNote.value = 'This browser cannot share its location. Enter the coordinates by hand.'
    return
  }
  geoNote.value = 'Asking for your location…'
  navigator.geolocation.getCurrentPosition(
    (pos) => {
      d.value.lat = pos.coords.latitude.toFixed(4)
      d.value.lon = pos.coords.longitude.toFixed(4)
      if (!d.value.label) d.value.label = 'Home'
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
</script>

<template>
  <SettingsPage title="Household">
    <UiCard v-if="household.conflict" variant="outlined" role="alert">
      <p>This page changed elsewhere. Reload to see the latest.</p>
      <template #actions><UiButton variant="tonal" @click="reload">Reload</UiButton></template>
    </UiCard>

    <UiCard>
      <template #header>Household</template>
      <UiTextField v-model="d.name" label="Name" name="name" required />
      <UiSelect v-model="d.tz" label="Time zone" :options="zoneOptions" />
      <UiSegmented
        :model-value="weekStart.value.value"
        label="First day of the week"
        :options="[
          { value: 'monday', label: 'Monday' },
          { value: 'sunday', label: 'Sunday' },
        ]"
        @update:model-value="weekStart.set($event as 'monday' | 'sunday')"
      />
    </UiCard>

    <UiCard>
      <template #header>Location (for weather)</template>
      <UiTextField v-model="d.label" label="Label" name="label" supporting="For example Home" />
      <UiTextField
        v-model="d.lat"
        label="Latitude"
        name="lat"
        :error="fieldErrors.lat"
        inputmode="decimal"
      />
      <UiTextField
        v-model="d.lon"
        label="Longitude"
        name="lon"
        :error="fieldErrors.lon"
        inputmode="decimal"
      />
      <p v-if="geoNote" class="note" role="status">{{ geoNote }}</p>
      <template #actions>
        <UiButton variant="text" icon="refresh" @click="useDeviceLocation"
          >Use this device's location</UiButton
        >
      </template>
    </UiCard>

    <UiCard>
      <template #header>Units and currency</template>
      <UiSegmented
        :model-value="units.value.value"
        label="Units"
        :options="[
          { value: 'metric', label: 'Metric (°C, km/h, mm)' },
          { value: 'imperial', label: 'Imperial (°F, mph, in)' },
        ]"
        @update:model-value="units.set($event as 'metric' | 'imperial')"
      />
      <UiTextField
        v-model="d.currency"
        label="Currency for bills (ISO code)"
        name="currency"
        maxlength="3"
        required
      />
    </UiCard>

    <UiCard>
      <template #header>Calendar window</template>
      <UiTextField v-model="d.past" label="Days of past events to keep" type="number" name="past" />
      <UiTextField v-model="d.future" label="Days ahead to sync" type="number" name="future" />
      <UiTextField
        v-model="d.retention"
        label="Days of weather history to keep"
        type="number"
        name="retention"
      />
      <UiSwitch
        :model-value="asBusy.value.value"
        label="Show adults' private events as “Busy” to children and on the dashboard"
        @update:model-value="asBusy.set($event)"
      />
    </UiCard>

    <UiCard>
      <template #header>Now list</template>
      <UiSwitch
        :model-value="override.value.value"
        label="Allow ticking off tasks on dashboards"
        @update:model-value="override.set($event)"
      />
      <UiTextField
        v-model="d.nowSize"
        label="How many tasks the Now list shows"
        type="number"
        name="now_list_size"
      />
      <p class="note">
        What floats to the top of Now: higher numbers count for more. Due date is how close or late
        a task is, priority is the high-priority flag, weather is a task made by a weather rule.
      </p>
      <UiTextField v-model="d.wDue" label="Weight: due date" type="number" name="w_due" />
      <UiTextField v-model="d.wFlag" label="Weight: priority" type="number" name="w_flag" />
      <UiTextField v-model="d.wWeather" label="Weight: weather" type="number" name="w_weather" />
    </UiCard>

    <UiCard>
      <template #header>Quiet hours (no notifications)</template>
      <UiTextField v-model="d.quietFrom" label="From" type="time" name="quiet_from" required />
      <UiTextField v-model="d.quietUntil" label="Until" type="time" name="quiet_until" required />
    </UiCard>

    <UiCard data-testid="mail-household">
      <template #header>Mail and approvals</template>
      <p class="note">
        Sources, categories and rules are under <RouterLink to="/settings/mail">Mail</RouterLink>;
        schools and clubs under <RouterLink to="/settings/orgs">Organizations</RouterLink>.
      </p>
      <UiSwitch
        :model-value="autoWrite.value.value"
        label="Add events found in mail to the calendar without asking (off: you approve each one)"
        @update:model-value="autoWrite.set($event)"
      />
      <template v-if="autoWrite.value.value">
        <UiTextField
          v-model="d.extractThreshold"
          label="Minimum confidence that it is an event (0 to 1)"
          type="number"
          name="extract_threshold"
        />
        <UiTextField
          v-model="d.routeThreshold"
          label="Minimum confidence for the calendar choice (0 to 1)"
          type="number"
          name="route_threshold"
        />
      </template>
      <UiTextField
        v-model="d.taskExpiry"
        label="Days before a suggested task expires"
        type="number"
        name="task_expiry_days"
      />
      <UiTextField
        v-model="d.contactExpiry"
        label="Days before a contact suggestion expires"
        type="number"
        name="contact_expiry_days"
      />
      <UiTextField
        v-model="d.female"
        label="Words for girls' groups (comma separated)"
        name="gender_female"
      />
      <UiTextField
        v-model="d.male"
        label="Words for boys' groups (comma separated)"
        name="gender_male"
      />
      <UiTextField
        v-model="d.billLead"
        label="Days before the due date that a bill's task is due"
        type="number"
        name="bill_lead_days"
      />
      <UiSelect v-model="d.policy" label="Keeping email content" :options="policyOptions" />
      <UiTextField
        v-model="d.examples"
        label="Examples kept per category"
        type="number"
        name="examples_per_category"
      />
      <UiTextField
        v-model="d.maxPending"
        label="Days to keep content of unprocessed mail"
        type="number"
        name="max_pending_days"
      />
    </UiCard>

    <UiCard>
      <template #header>Dashboard</template>
      <UiSwitch
        :model-value="pendingBadge.value.value"
        label="Show each adult how many approvals wait for them (a count only)"
        @update:model-value="pendingBadge.set($event)"
      />
    </UiCard>

    <UiSaveBar
      :dirty="form.dirty.value"
      :saving="form.saving.value"
      :error="form.error.value"
      @save="form.save"
      @discard="form.discard"
    />
  </SettingsPage>
</template>

<style scoped>
.note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>
