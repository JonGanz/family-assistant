<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api, type StatusView } from '../api/client'
import SettingsPage from '../app/SettingsPage.vue'
import { formatTime } from '../lib/time'
import { useHouseholdStore } from '../stores/household'
import { UiCard, UiChip, UiEmptyState, UiErrorState, UiIconButton, UiSkeleton } from '../ui'
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
.err {
  margin: 0;
  color: var(--ui-error);
}
</style>
