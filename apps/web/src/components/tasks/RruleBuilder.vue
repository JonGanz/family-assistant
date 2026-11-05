<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { UiChip, UiSelect, UiTextField } from '../../ui'
import { WEEKDAYS, buildRrule, parseSimple, type SimpleRule, type Weekday } from '../../lib/rrule'

const model = defineModel<string>({ required: true })

type Mode = SimpleRule['kind'] | 'raw'
const DAY_LABEL: Record<Weekday, string> = {
  MO: 'Mon',
  TU: 'Tue',
  WE: 'Wed',
  TH: 'Thu',
  FR: 'Fri',
  SA: 'Sat',
  SU: 'Sun',
}

const initial = parseSimple(model.value)
const mode = ref<Mode>(initial ? initial.kind : model.value.trim() ? 'raw' : 'weekly')
const interval = ref(initial?.interval ?? 1)
const weekdays = ref<Weekday[]>(initial?.kind === 'weekly' ? [...initial.weekdays] : ['SA'])
const day = ref(initial?.kind === 'monthly_day' ? initial.day : 1)
const nth = ref(initial?.kind === 'monthly_nth' ? initial.nth : 1)
const nthDay = ref<Weekday>(initial?.kind === 'monthly_nth' ? initial.weekday : 'MO')
const raw = ref(model.value)

const rule = computed<SimpleRule | null>(() => {
  const n = Math.max(1, Math.floor(interval.value) || 1)
  if (mode.value === 'weekly') return { kind: 'weekly', interval: n, weekdays: weekdays.value }
  if (mode.value === 'monthly_day') return { kind: 'monthly_day', interval: n, day: day.value }
  if (mode.value === 'monthly_nth')
    return { kind: 'monthly_nth', interval: n, nth: nth.value, weekday: nthDay.value }
  return null
})

watch([rule, raw, mode], () => {
  const next = rule.value ? buildRrule(rule.value) : raw.value
  if (next !== model.value) model.value = next
})

function toggleDay(d: Weekday) {
  weekdays.value = weekdays.value.includes(d)
    ? weekdays.value.filter((x) => x !== d)
    : [...weekdays.value, d]
}

const MODES = [
  { value: 'weekly', label: 'Weekly on days' },
  { value: 'monthly_day', label: 'Monthly on a day of the month' },
  { value: 'monthly_nth', label: 'Monthly on the nth weekday' },
  { value: 'raw', label: 'Custom (RRULE text)' },
]
const DAY_OPTIONS = [
  ...Array.from({ length: 31 }, (_, i) => ({ value: String(i + 1), label: String(i + 1) })),
  { value: '-1', label: 'Last day' },
]
const NTH_OPTIONS = [
  { value: '1', label: '1st' },
  { value: '2', label: '2nd' },
  { value: '3', label: '3rd' },
  { value: '4', label: '4th' },
  { value: '-1', label: 'Last' },
]
const WEEKDAY_OPTIONS = WEEKDAYS.map((d) => ({ value: d, label: DAY_LABEL[d] }))

function switchMode(m: Mode) {
  if (m === 'raw') raw.value = model.value
  mode.value = m
}
</script>

<template>
  <div class="rrule-builder">
    <UiSelect
      data-testid="mode"
      label="Repeat"
      :model-value="mode"
      :options="MODES"
      @update:model-value="switchMode($event as Mode)"
    />

    <UiTextField
      v-if="mode !== 'raw'"
      :model-value="String(interval)"
      data-testid="interval"
      type="number"
      min="1"
      :label="mode === 'weekly' ? 'Every N weeks' : 'Every N months'"
      @update:model-value="interval = Number($event)"
    />

    <div v-if="mode === 'weekly'" class="days" role="group" aria-label="Days">
      <UiChip
        v-for="d in WEEKDAYS"
        :key="d"
        variant="filter"
        :selected="weekdays.includes(d)"
        :data-testid="`day-${d}`"
        @click="toggleDay(d)"
      >
        {{ DAY_LABEL[d] }}
      </UiChip>
    </div>

    <UiSelect
      v-if="mode === 'monthly_day'"
      data-testid="month-day"
      label="Day of month"
      :model-value="String(day)"
      :options="DAY_OPTIONS"
      @update:model-value="day = Number($event)"
    />

    <template v-if="mode === 'monthly_nth'">
      <UiSelect
        data-testid="nth"
        label="Which"
        :model-value="String(nth)"
        :options="NTH_OPTIONS"
        @update:model-value="nth = Number($event)"
      />
      <UiSelect
        data-testid="nth-day"
        label="Weekday"
        :model-value="nthDay"
        :options="WEEKDAY_OPTIONS"
        @update:model-value="nthDay = $event as Weekday"
      />
    </template>

    <UiTextField
      v-if="mode === 'raw'"
      v-model="raw"
      data-testid="raw"
      label="RRULE"
      spellcheck="false"
    />
  </div>
</template>

<style scoped>
.rrule-builder {
  display: flex;
  flex-direction: column;
  gap: var(--ui-space-3);
}
.days {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
</style>
