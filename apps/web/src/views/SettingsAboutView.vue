<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type AboutView } from '../api/client'
import ErrorBanner from '../components/ErrorBanner.vue'
import PageHeader from '../components/PageHeader.vue'
import { useHouseholdStore } from '../stores/household'
import { buildsDiffer, builtAtText } from './aboutModel'
import { describeError } from './shared'

const household = useHouseholdStore()
const about = ref<AboutView | null>(null)
const error = ref('')

// Stamped into this bundle by vite.config.ts.
const web = { commit: __APP_COMMIT__, builtAt: __APP_BUILT_AT__ }
const stale = computed(() => buildsDiffer(about.value?.commit, web.commit))

onMounted(async () => {
  try {
    about.value = await api.about()
  } catch (e) {
    error.value = describeError(e, 'Could not load the version information.')
  }
})
</script>

<template>
  <section aria-labelledby="page-title">
    <PageHeader eyebrow="Settings" title="About" />
    <ErrorBanner v-if="error" :message="error" />
    <p v-if="stale" class="text-danger" role="status" data-testid="stale">
      This page was built from a different commit than the server. Reload without the cache
      (Shift+F5), or unregister the service worker in the browser's developer tools.
    </p>
    <template v-if="about">
      <h2>Server</h2>
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
    </template>
    <h2>This page</h2>
    <dl class="about" data-testid="web">
      <dt>Commit</dt>
      <dd>
        <code>{{ web.commit || 'unknown' }}</code>
      </dd>
      <dt>Built</dt>
      <dd>{{ builtAtText(web.builtAt, household.tz) }}</dd>
    </dl>
  </section>
</template>

<style scoped>
.about {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: var(--space-1, 0.25rem) var(--space-4, 1rem);
}
dt {
  color: var(--color-muted);
}
dd {
  margin: 0;
}
</style>
