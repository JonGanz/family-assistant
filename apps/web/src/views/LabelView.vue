<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import type { DecisionPointKey } from '../api/client'
import DecisionFix from '../components/mail/DecisionFix.vue'
import { LABEL_POINTS, valueOrDefault } from '../components/mail/decisions'
import { fmt } from '../components/mail/format'
import { useHouseholdStore } from '../stores/household'
import { useMailStore } from '../stores/mail'
import { useMembersStore } from '../stores/members'
import {
  UiButton,
  UiCard,
  UiEmptyState,
  UiErrorState,
  UiPage,
  UiProgress,
  UiSkeleton,
  UiSwitch,
  UiTopAppBar,
  useInstantSave,
  useSnackbar,
} from '../ui'
import { describeError } from './shared'

const mail = useMailStore()
const household = useHouseholdStore()
const members = useMembersStore()
const router = useRouter()
const snackbar = useSnackbar()
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
  try {
    await fn()
  } catch (e) {
    snackbar.show({ text: describeError(e) })
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
// 'In the evaluation sample' is a single switch: instant, with Undo (D-096).
const sample = ref(false)
watch(
  () => msg.value?.summary.in_eval_sample,
  (v) => (sample.value = !!v),
  { immediate: true },
)
const { set: setSample } = useInstantSave(
  sample,
  async (on) => {
    if (msg.value) await mail.setEvalSample(msg.value.summary.id, on)
  },
  { label: 'Evaluation sample updated' },
)
</script>
<template>
  <UiPage>
    <template #top>
      <UiTopAppBar title="Label mail" back @back="router.push('/mail')" />
    </template>

    <UiErrorState v-if="mail.labelError" :message="mail.labelError" @retry="mail.nextToLabel()" />
    <UiEmptyState
      v-else-if="mail.labelEmpty"
      icon="check"
      title="Nothing left to label right now."
    />
    <UiSkeleton v-else-if="!msg || !cur" :lines="5" />
    <template v-else>
      <div class="progress" data-testid="label-progress">
        <span>{{ cur.progress.labelled }} of {{ cur.progress.target }} labelled</span>
        <UiProgress :value="pct" label="Labelling progress" />
      </div>
      <UiCard variant="filled">
        <template #header>
          <h2 class="subject" data-testid="label-subject">{{ msg.summary.subject }}</h2>
        </template>
        <p class="note">
          {{ msg.summary.from_name || msg.summary.from_addr }} &middot;
          {{ fmt(msg.summary.received_at, household.tz) }}
        </p>
        <p class="snippet">{{ msg.summary.snippet ?? msg.body_text?.slice(0, 400) }}</p>
        <RouterLink :to="{ name: 'mail-detail', params: { id: msg.summary.id } }"
          >Full message</RouterLink
        >
      </UiCard>
      <UiCard variant="filled">
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
          <UiSwitch
            :model-value="sample"
            label="In the evaluation sample"
            @update:model-value="setSample"
          />
        </div>
      </UiCard>
      <div class="buttons">
        <UiButton variant="text" :disabled="busy" @click="next">Skip</UiButton>
        <UiButton :disabled="busy" icon="check" @click="looksRight">Looks right</UiButton>
      </div>
    </template>
  </UiPage>
</template>

<style scoped>
.progress {
  display: grid;
  gap: var(--ui-space-2);
  font: var(--ui-type-body-md);
}
.subject {
  margin: 0;
  font: var(--ui-type-title-md);
  overflow-wrap: anywhere;
}
.note {
  margin: 0 0 var(--ui-space-2);
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.snippet {
  margin: 0 0 var(--ui-space-2);
  overflow-wrap: anywhere;
  font: var(--ui-type-body-md);
}
.points {
  display: grid;
  gap: var(--ui-space-3);
}
.buttons {
  position: sticky;
  bottom: calc(var(--ui-bottom-offset, 0px) + env(safe-area-inset-bottom));
  z-index: 5;
  display: flex;
  justify-content: flex-end;
  gap: var(--ui-space-3);
  padding: var(--ui-space-3) 0;
  background: var(--ui-surface);
}
.buttons > * {
  flex: 1;
}
</style>
