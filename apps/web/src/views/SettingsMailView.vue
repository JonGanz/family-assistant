<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import type { NewMailSource } from '../api/mailSettings'
import SettingsPage from '../app/SettingsPage.vue'
import { useHouseholdStore } from '../stores/household'
import { useMailSettingsStore } from '../stores/mailSettings'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import { useSourcesStore } from '../stores/sources'
import { UiCard, UiSection, UiSheet } from '../ui'
import AddMailSourceForm from './parts/settings-mail/AddMailSourceForm.vue'
import CategoriesPanel from './parts/settings-mail/CategoriesPanel.vue'
import MailSourcesPanel from './parts/settings-mail/MailSourcesPanel.vue'
import PageFab from './parts/settings-mail/PageFab.vue'
import PageNotices from './parts/settings-mail/PageNotices.vue'
import RulesPanel from './parts/settings-mail/RulesPanel.vue'
import SignalsPanel from './parts/settings-mail/SignalsPanel.vue'
import { describeError } from './shared'

const store = useMailSettingsStore()
const members = useMembersStore()
const household = useHouseholdStore()
const sources = useSourcesStore()
const session = useSessionStore()
const error = ref('')
const adding = ref(false)

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
    if (await store.createSource(b)) adding.value = false
  } catch (e) {
    error.value = describeError(e, 'Could not add the source.')
    adding.value = false
  }
}
</script>

<template>
  <SettingsPage title="Mail">
    <PageNotices
      :load-error="store.error"
      :conflict="store.conflict"
      :error="error"
      @reload="store.load()"
    />
    <UiCard variant="outlined">
      <p>
        Approval, expiry, currency and retention settings are under
        <RouterLink to="/settings/household">Household</RouterLink>. Schools and clubs are under
        <RouterLink to="/settings/orgs">Organizations</RouterLink>.
      </p>
    </UiCard>

    <UiSection heading="Sources"
      ><MailSourcesPanel :members="members.items" :tz="household.tz"
    /></UiSection>
    <UiSection heading="Categories"><CategoriesPanel /></UiSection>
    <UiSection heading="Rules"><RulesPanel :members="members.items" /></UiSection>
    <UiSection
      heading="Built-in signals"
      supporting="Turn off a signal that guesses wrong for your household."
    >
      <SignalsPanel />
    </UiSection>

    <PageFab label="Add a mail source" @click="adding = true" />
    <UiSheet v-model="adding" title="Add a mail source">
      <AddMailSourceForm
        :members="members.items"
        :accounts="sources.accounts"
        :google-enabled="googleEnabled"
        :me-id="session.member?.id"
        @create="create"
      />
    </UiSheet>
  </SettingsPage>
</template>
