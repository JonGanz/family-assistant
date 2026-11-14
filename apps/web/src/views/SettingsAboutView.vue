<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type AboutView } from '../api/client'
import SettingsPage from '../app/SettingsPage.vue'
import { reloadApp } from '../lib/appUpdate'
import { useHouseholdStore } from '../stores/household'
import { UiButton, UiCard, UiErrorState, UiIcon, UiSkeleton } from '../ui'
import { buildsDiffer, builtAtText } from './aboutModel'
import { releaseNotes, sameVersion } from './releaseNotes'
import { describeError } from './shared'

const household = useHouseholdStore()
const about = ref<AboutView | null>(null)
const error = ref('')
const loading = ref(true)

// Stamped into this bundle by vite.config.ts.
const web = { commit: __APP_COMMIT__, builtAt: __APP_BUILT_AT__ }
const stale = computed(() => buildsDiffer(about.value?.commit, web.commit))
const notes = releaseNotes()
const SHOWN = 5
const showAll = ref(false)
const visibleNotes = computed(() => (showAll.value ? notes : notes.slice(0, SHOWN)))
const reloading = ref(false)
async function reload() {
  reloading.value = true
  await reloadApp()
}

async function load() {
  loading.value = true
  error.value = ''
  try {
    about.value = await api.about()
  } catch (e) {
    error.value = describeError(e, 'Could not load the version information.')
  } finally {
    loading.value = false
  }
}
onMounted(load)
</script>

<template>
  <SettingsPage title="About">
    <UiCard v-if="stale" variant="outlined" class="warn" role="status" data-testid="stale">
      <div class="warn__row">
        <UiIcon name="warning" />
        <p>
          This page was built from a different commit than the server. It reloads by itself the next
          time the app comes back to the foreground, or reload now.
        </p>
      </div>
    </UiCard>

    <div class="reload">
      <UiButton
        variant="tonal"
        icon="refresh"
        :disabled="reloading"
        data-testid="reload-app"
        @click="reload"
      >
        Reload app
      </UiButton>
      <p class="muted">
        Fetches the newest version. Use it if the app looks out of date; nothing you saved is lost.
      </p>
    </div>

    <UiErrorState v-if="error" :message="error" @retry="load" />
    <UiSkeleton v-else-if="loading" :lines="4" />
    <UiCard v-else-if="about">
      <template #header>Server</template>
      <dl class="about" data-testid="server">
        <dt>Version</dt>
        <dd>{{ about.version }}</dd>
        <dt>Commit</dt>
        <dd>
          <code>{{ about.commit ?? 'unknown' }}</code>
        </dd>
        <dt>Built</dt>
        <dd>{{ builtAtText(about.built_at, household.tz) }}</dd>
      </dl>
    </UiCard>

    <UiCard>
      <template #header>This page</template>
      <dl class="about" data-testid="web">
        <dt>Commit</dt>
        <dd>
          <code>{{ web.commit || 'unknown' }}</code>
        </dd>
        <dt>Built</dt>
        <dd>{{ builtAtText(web.builtAt, household.tz) }}</dd>
      </dl>
    </UiCard>

    <UiCard v-if="notes.length" data-testid="release-notes">
      <template #header>Release notes</template>
      <ol class="notes">
        <li
          v-for="n in visibleNotes"
          :key="n.version"
          class="notes__entry"
          :data-testid="`notes-${n.version}`"
        >
          <h3>
            {{ n.unreleased ? 'Unreleased' : `v${n.version}` }}
            <span v-if="n.date" class="muted">{{ n.date }}</span>
            <span
              v-if="sameVersion(about?.version, n.version)"
              class="notes__current"
              data-testid="notes-current"
            >
              You're on this version
            </span>
          </h3>
          <ul>
            <li v-for="(b, i) in n.bullets" :key="i">
              <template v-for="(s, j) in b" :key="j">
                <code v-if="s.kind === 'code'">{{ s.text }}</code>
                <strong v-else-if="s.kind === 'bold'">{{ s.text }}</strong>
                <template v-else>{{ s.text }}</template>
              </template>
            </li>
          </ul>
        </li>
      </ol>
      <UiButton
        v-if="notes.length > SHOWN"
        variant="text"
        data-testid="notes-toggle"
        @click="showAll = !showAll"
      >
        {{ showAll ? 'Show fewer' : `Show all (${notes.length})` }}
      </UiButton>
    </UiCard>
  </SettingsPage>
</template>

<style scoped>
.about {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: var(--ui-space-1) var(--ui-space-4);
  margin: 0;
}
dt {
  color: var(--ui-on-surface-variant);
}
dd {
  margin: 0;
  min-width: 0;
  overflow-wrap: anywhere;
}
.warn {
  border-color: var(--ui-warning);
}
.warn__row {
  display: flex;
  gap: var(--ui-space-3);
  align-items: flex-start;
}
.warn__row p {
  margin: 0;
}

.reload {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 12px;
}
.reload .muted {
  margin: 0;
}

.notes {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--ui-space-4);
}
.notes h3 {
  margin: 0 0 var(--ui-space-1);
  font: var(--ui-type-title-sm);
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--ui-space-2);
}
.notes h3 .muted {
  font: var(--ui-type-body-sm);
  color: var(--ui-on-surface-variant);
}
.notes ul {
  margin: 0;
  padding-left: var(--ui-space-5);
}
.notes__current {
  font: var(--ui-type-label-md);
  color: var(--ui-primary);
}
</style>
