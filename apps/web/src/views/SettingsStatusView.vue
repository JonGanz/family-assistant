<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type JobFailuresView, type StatusView } from '../api/client'
import SettingsPage from '../app/SettingsPage.vue'
import { copyText } from '../lib/copy'
import { formatTime } from '../lib/time'
import { useHouseholdStore } from '../stores/household'
import {
  UiButton,
  UiCard,
  UiChip,
  UiEmptyState,
  UiErrorState,
  UiIconButton,
  UiSkeleton,
  useSnackbar,
} from '../ui'
import { stateIcon } from './parts/settings-mail/stateIcon'
import { describeError } from './shared'

const household = useHouseholdStore()
const status = ref<StatusView | null>(null)
const error = ref('')

async function load() {
  try {
    status.value = await api.status()
    error.value = ''
  } catch (e) {
    error.value = describeError(e, 'Could not load the status.')
  }
}
onMounted(load)

const snackbar = useSnackbar()
const failures = ref<JobFailuresView | null>(null)
const failuresOpen = ref(false)
const failuresError = ref('')
const fallbackText = ref('')
const retrying = ref('')
const openGroups = ref<Set<string>>(new Set())

const problems = computed(() => (failures.value?.dead ?? 0) + (failures.value?.retrying ?? 0))
const groupKey = (g: { kind: string; status: string; error?: string | null }) =>
  `${g.kind}|${g.status}|${g.error ?? ''}`

async function loadFailures() {
  try {
    failures.value = await api.failedJobs()
    failuresError.value = ''
  } catch (e) {
    failuresError.value = describeError(e, 'Could not load the job errors.')
  }
}
async function toggleFailures() {
  failuresOpen.value = !failuresOpen.value
  if (failuresOpen.value) await loadFailures()
}
function toggleGroup(key: string) {
  const next = new Set(openGroups.value)
  if (!next.delete(key)) next.add(key)
  openGroups.value = next
}
function jobsOf(g: { kind: string; status: string; error?: string | null }) {
  return (failures.value?.jobs ?? []).filter(
    (j) =>
      j.kind === g.kind &&
      j.status === g.status &&
      (j.last_error ?? '').split('\n')[0] === (g.error ?? ''),
  )
}

function report(f: JobFailuresView): string {
  const out = [
    'Family Assistant job errors',
    `App version: ${f.app_version}`,
    `Report time: ${new Date().toISOString()}`,
    `Failed: ${f.dead}, retrying: ${f.retrying}`,
  ]
  if (f.llm) {
    out.push(
      `AI: last failed call ${f.llm.at} (${f.llm.purpose}, model ${f.llm.model}, ${f.llm.error_kind ?? 'unknown error'}); ${f.llm.failed_24h} of ${f.llm.calls_24h} calls failed in 24 h`,
    )
  }
  out.push('', 'Groups')
  for (const g of f.groups) {
    out.push(
      `- ${g.kind} [${g.status === 'dead' ? 'failed' : 'retrying'}] x${g.count}: ${g.error ?? '(no error text)'} (last ${g.last_at})`,
    )
  }
  out.push('', 'Jobs')
  for (const j of f.jobs) {
    const refs = Object.entries(j.refs)
      .map(([k, v]) => `${k}=${v}`)
      .join(' ')
    out.push(
      `- ${j.id} ${j.kind} [${j.status === 'dead' ? 'failed' : 'retrying'}] attempts ${j.attempts}/${j.max_attempts} created ${j.created_at} failed ${j.updated_at} next run ${j.run_at}${refs ? ` ${refs}` : ''}`,
      `  ${(j.last_error ?? '(no error text)').replace(/\n/g, '\n  ')}`,
    )
  }
  return out.join('\n')
}
async function copyReport() {
  if (!failures.value) return
  const text = report(failures.value)
  if (await copyText(text)) {
    fallbackText.value = ''
    snackbar.show({ text: 'Copied the job errors.' })
  } else {
    fallbackText.value = text
  }
}
async function retry(kind: string) {
  retrying.value = kind
  try {
    const r = await api.retryJobs(kind)
    snackbar.show({ text: `Requeued ${r.requeued} job${r.requeued === 1 ? '' : 's'}.` })
    await Promise.all([load(), loadFailures()])
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not requeue the jobs.') })
  } finally {
    retrying.value = ''
  }
}

function when(iso: string | null | undefined): string {
  if (!iso) return 'never'
  const mins = Math.round((Date.now() - new Date(iso).getTime()) / 60_000)
  const ago =
    mins < 1 ? 'just now' : mins < 90 ? `${mins} min ago` : `${Math.round(mins / 60)} h ago`
  return `${formatTime(iso, household.tz)} (${ago})`
}
</script>

<template>
  <SettingsPage title="Status">
    <template #actions>
      <UiIconButton icon="refresh" label="Refresh" @click="load" />
    </template>
    <UiErrorState v-if="error" :message="error" @retry="load" />
    <UiSkeleton v-else-if="!status" :lines="4" />
    <template v-else>
      <UiCard>
        <template #header>Calendar sources</template>
        <UiEmptyState v-if="!status.sources.length" icon="calendar" title="No sources." />
        <div v-for="s in status.sources" :key="s.id" class="row">
          <div class="row__head">
            <strong class="row__name">{{ s.name }}</strong>
            <UiChip :icon="stateIcon(s.state)">{{ s.state }}</UiChip>
          </div>
          <p class="meta">{{ s.kind }} · last sync {{ when(s.last_synced_at) }}</p>
          <p v-if="s.last_error" class="err">{{ s.last_error }}</p>
        </div>
      </UiCard>
      <UiCard>
        <template #header>Weather</template>
        <div class="row__head">
          <UiChip :icon="status.weather.ok ? 'success' : 'error'">{{
            status.weather.ok ? 'ok' : 'failing'
          }}</UiChip>
          <span>last fetch {{ when(status.weather.fetched_at) }}</span>
        </div>
      </UiCard>
      <UiCard>
        <template #header>Backup</template>
        <div class="row__head">
          <UiChip :icon="status.backup.ok ? 'success' : 'warning'">{{
            status.backup.ok ? 'ok' : 'attention'
          }}</UiChip>
          <span>last backup {{ when(status.backup.last_at) }}</span>
        </div>
      </UiCard>
      <UiCard>
        <template #header>Background jobs</template>
        <p data-testid="queue">
          {{ status.queue.ready }} waiting, {{ status.queue.running }} running,
          <span :class="{ err: status.queue.dead > 0 }">{{ status.queue.dead }} failed</span>
        </p>
        <UiButton
          v-if="status.queue.dead > 0 || failuresOpen"
          variant="text"
          :icon="failuresOpen ? 'collapse' : 'expand'"
          data-testid="failures-toggle"
          :aria-expanded="failuresOpen"
          @click="toggleFailures"
        >
          {{ failuresOpen ? 'Hide errors' : 'Show errors' }}
        </UiButton>
        <div v-if="failuresOpen" class="failures" data-testid="failures">
          <p v-if="failuresError" class="err">{{ failuresError }}</p>
          <UiSkeleton v-else-if="!failures" :lines="3" />
          <template v-else>
            <p data-testid="failures-summary">
              {{ failures.dead }} failed{{
                failures.retrying ? `, ${failures.retrying} retrying` : ''
              }}
            </p>
            <p v-if="failures.llm" class="meta" data-testid="llm-problem">
              AI: last failed call {{ when(failures.llm.at) }} ({{ failures.llm.purpose }},
              {{ failures.llm.error_kind ?? 'unknown error' }}); {{ failures.llm.failed_24h }} of
              {{ failures.llm.calls_24h }} calls failed in the last 24 h.
            </p>
            <p v-if="!problems" class="meta">No failed or retrying jobs.</p>
            <div v-for="g in failures.groups" :key="groupKey(g)" class="group">
              <div class="row__head">
                <UiChip :icon="g.status === 'dead' ? 'error' : 'warning'">{{
                  g.status === 'dead' ? 'failed' : 'retrying'
                }}</UiChip>
                <strong class="row__name">{{ g.kind }} ×{{ g.count }}</strong>
                <UiButton
                  v-if="g.status === 'dead'"
                  variant="text"
                  icon="refresh"
                  :loading="retrying === g.kind"
                  data-testid="retry"
                  @click="retry(g.kind)"
                  >Retry</UiButton
                >
                <UiIconButton
                  :icon="openGroups.has(groupKey(g)) ? 'collapse' : 'expand'"
                  label="Show jobs"
                  data-testid="group-toggle"
                  @click="toggleGroup(groupKey(g))"
                />
              </div>
              <p class="err">{{ g.error ?? '(no error text)' }}</p>
              <p class="meta">last {{ when(g.last_at) }}</p>
              <ul v-if="openGroups.has(groupKey(g))" class="jobs">
                <li v-for="j in jobsOf(g)" :key="j.id" class="meta" data-testid="job">
                  {{ j.id }} · attempts {{ j.attempts }}/{{ j.max_attempts }} · failed
                  {{ when(j.updated_at) }}
                  <template v-for="(v, k) in j.refs" :key="k"> · {{ k }} {{ v }}</template>
                  <pre v-if="j.last_error" class="detail">{{ j.last_error }}</pre>
                </li>
              </ul>
            </div>
            <UiButton variant="tonal" icon="copy" data-testid="copy-report" @click="copyReport"
              >Copy details</UiButton
            >
            <textarea
              v-if="fallbackText"
              class="fallback"
              readonly
              rows="8"
              :value="fallbackText"
              aria-label="Job error details, press Ctrl+C to copy"
              @focus="($event.target as HTMLTextAreaElement).select()"
            ></textarea>
          </template>
        </div>
      </UiCard>
    </template>
  </SettingsPage>
</template>

<style scoped>
.row {
  display: grid;
  gap: var(--ui-space-1);
}
.row__head {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.row__name {
  flex: 1;
}
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.failures,
.group {
  display: grid;
  gap: var(--ui-space-2);
  margin-top: var(--ui-space-2);
}
.jobs {
  margin: 0;
  padding-left: var(--ui-space-4);
  display: grid;
  gap: var(--ui-space-2);
}
.detail,
.fallback {
  margin: 0;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font: var(--ui-type-body-sm);
}
.fallback {
  width: 100%;
}
.err {
  margin: 0;
  color: var(--ui-error);
}
</style>
