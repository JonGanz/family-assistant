<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import type { NewMailSource } from '../api/mailSettings'
import ErrorBanner from '../components/ErrorBanner.vue'
import { useHouseholdStore } from '../stores/household'
import { useMailSettingsStore } from '../stores/mailSettings'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import { useSourcesStore } from '../stores/sources'
import AddMailSourceForm from './parts/settings-mail/AddMailSourceForm.vue'
import CategoriesPanel from './parts/settings-mail/CategoriesPanel.vue'
import MailSourcesPanel from './parts/settings-mail/MailSourcesPanel.vue'
import RulesPanel from './parts/settings-mail/RulesPanel.vue'
import SignalsPanel from './parts/settings-mail/SignalsPanel.vue'
import { describeError } from './shared'
import './parts/settings.css'

const store = useMailSettingsStore()
const members = useMembersStore()
const household = useHouseholdStore()
const sources = useSourcesStore()
const session = useSessionStore()
const error = ref('')

const googleEnabled = computed(() => household.view?.google_oauth_enabled ?? false)

onMounted(() => {
  void store.load()
  void members.load()
  void sources.load()
  if (!household.loaded) void household.load()
})

async function create(b: NewMailSource) {
  error.value = ''
  try {
    await store.createSource(b)
  } catch (e) {
    error.value = describeError(e, 'Could not add the source.')
  }
}
</script>

<template>
  <section>
    <h1>Mail</h1>
    <p class="settings-note">
      Approval, expiry, currency and retention settings are under
      <RouterLink to="/settings/household">Household</RouterLink>. Schools and clubs are under
      <RouterLink to="/settings/orgs">Organizations</RouterLink>.
    </p>
    <ErrorBanner v-if="store.error" :message="store.error" />
    <ErrorBanner v-if="store.conflict" conflict @reload="store.load()" />
    <ErrorBanner v-if="error" :message="error" />

    <h2>Sources</h2>
    <MailSourcesPanel :members="members.items" :tz="household.tz" />
    <AddMailSourceForm
      :members="members.items"
      :accounts="sources.accounts"
      :google-enabled="googleEnabled"
      :me-id="session.member?.id"
      @create="create"
    />

    <h2>Categories</h2>
    <CategoriesPanel />

    <h2>Rules</h2>
    <RulesPanel :members="members.items" />

    <h2>Built-in signals</h2>
    <p class="settings-note">Turn off a signal that guesses wrong for your household.</p>
    <SignalsPanel />
  </section>
</template>
