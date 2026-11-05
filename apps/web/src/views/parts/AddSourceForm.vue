<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { api, type NewSource, type OAuthAccountView, type SourceTestResult } from '../../api/client'
import { formatRange } from '../../lib/time'
import { GOOGLE_NEEDS_HTTPS, isSecure } from '../../lib/secure'
import { useSourcesStore } from '../../stores/sources'
import { UiButton, UiCard, UiSegmented, UiSelect, UiTextField } from '../../ui'
import { describeError } from '../shared'

const props = defineProps<{
  googleEnabled: boolean
  accounts: OAuthAccountView[]
  tz: string
  /** Same-origin path to return to after Google's consent screen. */
  returnTo: string
}>()
const emit = defineEmits<{ created: [] }>()

type Kind = 'google' | 'caldav' | 'ics'
const sources = useSourcesStore()
const secure = isSecure()
const kind = ref<Kind>(props.googleEnabled && secure ? 'google' : 'ics')
const form = reactive({ name: '', url: '', username: '', password: '', account: '' })
const result = ref<SourceTestResult | null>(null)
const kindOptions = computed(() => [
  ...(props.googleEnabled && secure ? [{ value: 'google', label: 'Google' }] : []),
  { value: 'caldav', label: 'CalDAV' },
  { value: 'ics', label: 'ICS link' },
])
const accountOptions = computed(() => [
  { value: '', label: 'Choose an account', disabled: true },
  ...usableAccounts.value.map((a) => ({ value: a.id, label: a.email })),
])
const addDisabled = computed(() => busy.value || (kind.value === 'google' && !form.account))
const goGoogle = () => (window.location.href = googleStart.value)
const error = ref('')
const busy = ref(false)

const usableAccounts = computed(() => props.accounts.filter((a) => a.state === 'ok'))
const googleStart = computed(() => api.googleStartUrl(props.returnTo))

function body(): NewSource | null {
  const name = form.name.trim() || 'Calendar'
  if (kind.value === 'google') {
    return form.account ? { kind: 'google', name, oauth_account: form.account } : null
  }
  if (kind.value === 'ics') return { kind: 'ics', name, url: form.url.trim() }
  return {
    kind: 'caldav',
    name,
    url: form.url.trim(),
    username: form.username,
    password: form.password,
  }
}

function pick(k: string) {
  kind.value = k as Kind
  result.value = null
  error.value = ''
}

async function test() {
  const b = body()
  if (!b) return
  busy.value = true
  error.value = ''
  result.value = null
  try {
    result.value = await api.testSource(b)
  } catch (e) {
    error.value = describeError(e, 'The test failed.')
  } finally {
    busy.value = false
  }
}

async function add() {
  const b = body()
  if (!b) return
  busy.value = true
  error.value = ''
  try {
    await sources.create(b)
    // Credentials are write-only: clear them as soon as they are sent.
    Object.assign(form, { name: '', url: '', username: '', password: '' })
    result.value = null
    emit('created')
  } catch (e) {
    error.value = describeError(e, 'Could not add the source.')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="add">
    <UiSegmented
      :model-value="kind"
      label="Source type"
      :options="kindOptions"
      @update:model-value="pick"
    />

    <p v-if="!secure" class="muted" data-testid="google-needs-https">
      Google: {{ GOOGLE_NEEDS_HTTPS }}. CalDAV and ICS links work over plain HTTP.
    </p>

    <form class="add__form" autocomplete="off" @submit.prevent="add">
      <UiTextField v-model="form.name" label="Name" name="source_name" />

      <template v-if="kind === 'google'">
        <UiSelect v-model="form.account" label="Google account" :options="accountOptions" />
        <UiButton variant="tonal" @click="goGoogle">Connect a Google account</UiButton>
      </template>

      <template v-else>
        <UiTextField
          v-model="form.url"
          :label="kind === 'ics' ? 'Calendar link (https://…)' : 'CalDAV server URL'"
          name="source_url"
          type="url"
          required
          supporting="The link or password is stored encrypted and is never shown again."
        />
        <template v-if="kind === 'caldav'">
          <UiTextField v-model="form.username" label="Username" name="source_user" required />
          <UiTextField
            v-model="form.password"
            label="Password"
            name="source_password"
            type="password"
            required
            autocomplete="new-password"
          />
        </template>
      </template>

      <div class="add__row">
        <UiButton v-if="kind !== 'google'" variant="tonal" :disabled="busy" @click="test">
          {{ kind === 'ics' ? 'Test' : 'Test & discover' }}
        </UiButton>
        <UiButton type="submit" :disabled="addDisabled">Add</UiButton>
      </div>
      <UiCard v-if="error" variant="outlined" role="alert">{{ error }}</UiCard>
    </form>

    <UiCard v-if="result" variant="outlined" data-testid="test-result">
      <p v-if="result.ok">
        It works. Found {{ result.calendars.length }}
        {{ result.calendars.length === 1 ? 'calendar' : 'calendars' }}.
      </p>
      <p v-else role="alert">{{ result.error ?? 'The test failed.' }}</p>
      <ul v-if="result.ok">
        <li v-for="c in result.calendars" :key="c.remote_id">{{ c.name }}</li>
      </ul>
      <template v-if="result.ok && result.sample_events.length">
        <h3>Next events</h3>
        <ul>
          <li v-for="e in result.sample_events" :key="e.id">
            {{ e.start_date ?? e.start_at.slice(0, 10) }} · {{ formatRange(e, tz) }} · {{ e.title }}
          </li>
        </ul>
      </template>
    </UiCard>
  </section>
</template>

<style scoped>
.add,
.add__form {
  display: grid;
  gap: var(--ui-space-3);
}
.add__row {
  display: flex;
  gap: var(--ui-space-2);
}
.muted {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>
