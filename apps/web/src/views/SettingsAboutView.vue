<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type AboutView } from '../api/client'
import SettingsPage from '../app/SettingsPage.vue'
import { useHouseholdStore } from '../stores/household'
import { UiCard, UiErrorState, UiIcon, UiSkeleton } from '../ui'
import { buildsDiffer, builtAtText } from './aboutModel'
import { describeError } from './shared'

const household = useHouseholdStore()
const about = ref<AboutView | null>(null)
const error = ref('')
const loading = ref(true)

// Stamped into this bundle by vite.config.ts.
const web = { commit: __APP_COMMIT__, builtAt: __APP_BUILT_AT__ }
const stale = computed(() => buildsDiffer(about.value?.commit, web.commit))

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
          This page was built from a different commit than the server. Reload without the cache
          (Shift+F5), or unregister the service worker in the browser's developer tools.
        </p>
      </div>
    </UiCard>

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
</style>
