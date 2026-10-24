<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api, type StatusView } from '../api/client'
import ErrorBanner from '../components/ErrorBanner.vue'
import { formatTime } from '../lib/time'
import { useHouseholdStore } from '../stores/household'
import { describeError } from './shared'
import './parts/settings.css'

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

function when(iso: string | null | undefined): string {
  if (!iso) return 'never'
  const mins = Math.round((Date.now() - new Date(iso).getTime()) / 60_000)
  const ago =
    mins < 1 ? 'just now' : mins < 90 ? `${mins} min ago` : `${Math.round(mins / 60)} h ago`
  return `${formatTime(iso, household.tz)} (${ago})`
}
</script>

<template>
  <section>
    <h1>Status</h1>
    <ErrorBanner v-if="error" :message="error" />
    <button type="button" @click="load">Refresh</button>
    <template v-if="status">
      <h2>Calendar sources</h2>
      <ul class="settings-list">
        <li v-for="s in status.sources" :key="s.id">
          <strong>{{ s.name }}</strong>
          <span class="badge" :class="s.state">{{ s.state }}</span>
          <span class="settings-note"> {{ s.kind }} · last sync {{ when(s.last_synced_at) }}</span>
          <p v-if="s.last_error" class="settings-error">{{ s.last_error }}</p>
        </li>
        <li v-if="!status.sources.length" class="settings-note">No sources.</li>
      </ul>
      <h2>Weather</h2>
      <p>
        <span class="badge" :class="status.weather.ok ? 'ok' : 'error'">
          {{ status.weather.ok ? 'ok' : 'failing' }}
        </span>
        last fetch {{ when(status.weather.fetched_at) }}
      </p>
      <h2>Backup</h2>
      <p>
        <span class="badge" :class="status.backup.ok ? 'ok' : 'error'">
          {{ status.backup.ok ? 'ok' : 'attention' }}
        </span>
        last backup {{ when(status.backup.last_at) }}
      </p>
      <h2>Background jobs</h2>
      <p data-testid="queue">
        {{ status.queue.ready }} waiting, {{ status.queue.running }} running,
        <span :class="{ 'settings-error': status.queue.dead > 0 }"
          >{{ status.queue.dead }} failed</span
        >
      </p>
    </template>
  </section>
</template>
