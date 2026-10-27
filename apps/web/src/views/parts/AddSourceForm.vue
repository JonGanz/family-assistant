<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { api, type NewSource, type OAuthAccountView, type SourceTestResult } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { formatRange } from '../../lib/time'
import { GOOGLE_NEEDS_HTTPS, isSecure } from '../../lib/secure'
import { useSourcesStore } from '../../stores/sources'
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

function pick(k: Kind) {
  kind.value = k
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
  <section>
    <h2>Add a calendar source</h2>
    <div class="kinds" role="group" aria-label="Source type">
      <button
        v-if="googleEnabled && secure"
        type="button"
        :aria-pressed="kind === 'google'"
        @click="pick('google')"
      >
        Google
      </button>
      <button type="button" :aria-pressed="kind === 'caldav'" @click="pick('caldav')">
        CalDAV
      </button>
      <button type="button" :aria-pressed="kind === 'ics'" @click="pick('ics')">ICS link</button>
    </div>

    <p v-if="!secure" class="muted" data-testid="google-needs-https">
      Google: {{ GOOGLE_NEEDS_HTTPS }}. CalDAV and ICS links work over plain HTTP.
    </p>

    <form class="form" autocomplete="off" @submit.prevent="add">
      <label>Name <input v-model="form.name" name="source_name" placeholder="Family" /></label>

      <template v-if="kind === 'google'">
        <label>
          Google account
          <select v-model="form.account" name="oauth_account">
            <option value="" disabled>Choose an account</option>
            <option v-for="a in usableAccounts" :key="a.id" :value="a.id">{{ a.email }}</option>
          </select>
        </label>
        <a class="button" :href="googleStart">Connect a Google account</a>
      </template>

      <template v-else>
        <label>
          {{ kind === 'ics' ? 'Calendar link (https://…)' : 'CalDAV server URL' }}
          <input v-model="form.url" name="source_url" type="url" required autocomplete="off" />
        </label>
        <template v-if="kind === 'caldav'">
          <label>
            Username
            <input v-model="form.username" name="source_user" required autocomplete="off" />
          </label>
          <label>
            Password
            <input
              v-model="form.password"
              name="source_password"
              type="password"
              required
              autocomplete="new-password"
            />
          </label>
        </template>
        <p class="muted">The link or password is stored encrypted and is never shown again.</p>
      </template>

      <div class="row">
        <button v-if="kind !== 'google'" type="button" :disabled="busy" @click="test">
          {{ kind === 'ics' ? 'Test' : 'Test & discover' }}
        </button>
        <button type="submit" :disabled="busy || (kind === 'google' && !form.account)">Add</button>
      </div>
      <ErrorBanner v-if="error" :message="error" />
    </form>

    <div v-if="result" class="result" data-testid="test-result">
      <p v-if="result.ok" class="text-success">
        It works. Found {{ result.calendars.length }}
        {{ result.calendars.length === 1 ? 'calendar' : 'calendars' }}.
      </p>
      <p v-else class="text-danger" role="alert">{{ result.error ?? 'The test failed.' }}</p>
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
    </div>
  </section>
</template>

<style scoped>
.kinds,
.row {
  display: flex;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}
button[aria-pressed='true'] {
  background: var(--color-accent);
  color: var(--color-bg);
}
.result {
  margin-top: var(--space-3);
}
</style>
