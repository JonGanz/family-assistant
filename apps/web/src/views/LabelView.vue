<script setup lang="ts">
import PageHeader from '../components/PageHeader.vue'
import StateBlock from '../components/StateBlock.vue'
import ErrorBanner from '../components/ErrorBanner.vue'
import { computed, onMounted, ref } from 'vue'
import type { DecisionPointKey } from '../api/client'
import DecisionFix from '../components/mail/DecisionFix.vue'
import { LABEL_POINTS, valueOrDefault } from '../components/mail/decisions'
import { fmt } from '../components/mail/format'
import { useHouseholdStore } from '../stores/household'
import { useMailStore } from '../stores/mail'
import { useMembersStore } from '../stores/members'
import { describeError } from './shared'

const mail = useMailStore()
const household = useHouseholdStore()
const members = useMembersStore()
const error = ref('')
const busy = ref(false)

const cur = computed(() => mail.label)
const msg = computed(() => cur.value?.message)
const pct = computed(() => {
  const p = cur.value?.progress
  return p && p.target > 0 ? Math.min(100, Math.round((p.labelled / p.target) * 100)) : 0
})

onMounted(() => {
  void members.load()
  void mail.loadFilters()
  void mail.nextToLabel()
})

async function run(fn: () => Promise<void>) {
  busy.value = true
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e)
  } finally {
    busy.value = false
  }
}
const fixPoint = (p: DecisionPointKey, v: unknown) =>
  run(() => mail.labelPoint({ decision_point: p, value: v }))

/** One tap records all four decision points as currently shown, then fetches the next message. */
function looksRight() {
  const m = msg.value
  if (!m) return
  return run(async () => {
    for (const p of LABEL_POINTS) {
      await mail.labelPoint({
        decision_point: p,
        value: valueOrDefault(m.decisions, p, m.summary.members),
      })
    }
    await mail.nextToLabel(m.summary.id)
  })
}
const next = () => msg.value && run(() => mail.nextToLabel(msg.value!.summary.id))
const toggleSample = (e: Event) =>
  msg.value &&
  run(() => mail.setEvalSample(msg.value!.summary.id, (e.target as HTMLInputElement).checked))
</script>

<template>
  <section>
    <PageHeader title="Label mail" back="/mail" />
    <StateBlock
      :error="mail.labelError"
      :empty="mail.labelEmpty"
      empty-text="Nothing left to label right now."
    />
    <ErrorBanner v-if="error" :message="error" />
    <template v-if="msg && cur">
      <label class="progress">
        {{ cur.progress.labelled }} of {{ cur.progress.target }} labelled
        <progress :value="pct" max="100">{{ pct }}%</progress>
      </label>
      <article class="card">
        <h2>{{ msg.summary.subject }}</h2>
        <p class="muted">
          {{ msg.summary.from_name || msg.summary.from_addr }} &middot;
          {{ fmt(msg.summary.received_at, household.tz) }}
        </p>
        <p class="snippet">{{ msg.summary.snippet ?? msg.body_text?.slice(0, 400) }}</p>
        <RouterLink :to="{ name: 'mail-detail', params: { id: msg.summary.id } }"
          >Full message</RouterLink
        >
      </article>
      <div class="points">
        <DecisionFix
          v-for="p in LABEL_POINTS"
          :key="p"
          :point="p"
          :decisions="msg.decisions"
          :categories="mail.categories"
          :members="members.items"
          open
          @fix="(v) => fixPoint(p, v)"
        />
      </div>
      <label class="field-inline">
        <input type="checkbox" :checked="msg.summary.in_eval_sample" @change="toggleSample" />
        In the evaluation sample
      </label>
      <div class="buttons">
        <button type="button" class="btn-primary" :disabled="busy" @click="looksRight">
          Looks right
        </button>
        <button type="button" class="btn-ghost" :disabled="busy" @click="next">Skip</button>
      </div>
    </template>
  </section>
</template>

<style scoped>
.progress {
  display: grid;
  gap: var(--space-1);
  margin-bottom: var(--space-3);
}
.progress progress {
  width: 100%;
}
.card {
  margin-bottom: var(--space-3);
}
.snippet {
  overflow-wrap: anywhere;
}
.points {
  display: grid;
  gap: var(--space-3);
  margin-bottom: var(--space-3);
}
.buttons {
  display: flex;
  gap: var(--space-3);
  position: sticky;
  bottom: 0;
  padding: var(--space-3) 0;
  background: var(--color-bg);
}
.buttons button {
  flex: 1;
  min-height: 52px;
}
</style>
