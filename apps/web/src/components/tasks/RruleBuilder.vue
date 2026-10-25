<script setup lang="ts">
import { computed, ref, watch } from 'vue'
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

function switchMode(m: Mode) {
  if (m === 'raw') raw.value = model.value
  mode.value = m
}
</script>

<template>
  <div class="rrule-builder">
    <label>
      Repeat
      <select
        data-testid="mode"
        :value="mode"
        @change="switchMode(($event.target as HTMLSelectElement).value as Mode)"
      >
        <option value="weekly">Weekly on days</option>
        <option value="monthly_day">Monthly on a day of the month</option>
        <option value="monthly_nth">Monthly on the nth weekday</option>
        <option value="raw">Custom (RRULE text)</option>
      </select>
    </label>

    <template v-if="mode !== 'raw'">
      <label>
        Every
        <input v-model.number="interval" data-testid="interval" type="number" min="1" />
        {{ mode === 'weekly' ? 'week(s)' : 'month(s)' }}
      </label>
    </template>

    <fieldset v-if="mode === 'weekly'" class="days">
      <legend>Days</legend>
      <label v-for="d in WEEKDAYS" :key="d" class="day">
        <input
          type="checkbox"
          :data-testid="`day-${d}`"
          :checked="weekdays.includes(d)"
          @change="toggleDay(d)"
        />
        {{ DAY_LABEL[d] }}
      </label>
    </fieldset>

    <label v-if="mode === 'monthly_day'">
      Day of month
      <select v-model.number="day" data-testid="month-day">
        <option v-for="n in 31" :key="n" :value="n">{{ n }}</option>
        <option :value="-1">Last day</option>
      </select>
    </label>

    <template v-if="mode === 'monthly_nth'">
      <label>
        Which
        <select v-model.number="nth" data-testid="nth">
          <option :value="1">1st</option>
          <option :value="2">2nd</option>
          <option :value="3">3rd</option>
          <option :value="4">4th</option>
          <option :value="-1">Last</option>
        </select>
      </label>
      <label>
        Weekday
        <select v-model="nthDay" data-testid="nth-day">
          <option v-for="d in WEEKDAYS" :key="d" :value="d">{{ DAY_LABEL[d] }}</option>
        </select>
      </label>
    </template>

    <label v-if="mode === 'raw'">
      RRULE
      <input v-model="raw" data-testid="raw" type="text" spellcheck="false" />
    </label>
  </div>
</template>

<style scoped>
.rrule-builder {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
input[type='number'] {
  width: 5rem;
}
.days {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}
.day {
  min-height: var(--min-touch, 48px);
}
</style>
