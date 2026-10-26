<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { OAuthAccountView } from '../../../api/client'
import { mailApi, type NewMailSource } from '../../../api/mailSettings'
import type { Member } from '../../../api/client'
import { describeError } from '../../shared'

const props = defineProps<{
  members: Member[]
  accounts: OAuthAccountView[]
  googleEnabled: boolean
  meId?: string
}>()
const emit = defineEmits<{ create: [b: NewMailSource] }>()

const RETURN_TO = '/settings/mail'
const provider = ref<'gmail' | 'imap'>('gmail')
const form = reactive({
  name: '',
  account: '',
  host: '',
  port: 993,
  username: '',
  password: '',
  folder: 'INBOX',
  archive: '',
  owners: [] as string[],
  readOnly: true,
  backfill: 30,
  query: '',
})
const testNote = ref('')
const testOk = ref(false)
const error = ref('')

const adults = computed(() => props.members.filter((m) => m.role === 'adult'))
const gmailAccounts = computed(() =>
  props.accounts.filter((a) => a.scopes.some((s) => s.includes('/auth/gmail.'))),
)
const needsScope = computed(() => provider.value === 'gmail' && gmailAccounts.value.length === 0)
const isGmailAddress = computed(() => /@(gmail|googlemail)\.com$/i.test(form.username))

function onUsername() {
  if (isGmailAddress.value) {
    if (!form.host) form.host = 'imap.gmail.com'
    if (!form.archive) form.archive = '[Gmail]/All Mail'
  }
}

async function test() {
  testNote.value = ''
  testOk.value = false
  try {
    const r = await mailApi.testImap({
      host: form.host,
      port: form.port,
      username: form.username,
      password: form.password,
    })
    testOk.value = r.ok
    testNote.value = r.ok
      ? `Connected${r.gmail_ext ? ' (Gmail labels available)' : ''}. ${r.folders.length} folders found.`
      : (r.error ?? 'Could not connect.')
    if (r.ok && !form.archive && r.folders.includes('[Gmail]/All Mail'))
      form.archive = '[Gmail]/All Mail'
  } catch (e) {
    testNote.value = describeError(e, 'The test failed.')
  }
}

function submit() {
  error.value = ''
  const owners = form.owners.length ? form.owners : props.meId ? [props.meId] : []
  if (!owners.length) {
    error.value = 'Pick at least one owner.'
    return
  }
  const base = {
    name: form.name.trim(),
    owners,
    read_only: form.readOnly,
    backfill_days: form.backfill,
    query: form.query.trim() || null,
  }
  if (provider.value === 'gmail') {
    if (!form.account) {
      error.value = 'Choose a Google account.'
      return
    }
    emit('create', { ...base, provider: 'gmail', oauth_account: form.account })
  } else {
    emit('create', {
      ...base,
      provider: 'imap',
      imap: {
        host: form.host.trim(),
        port: form.port,
        username: form.username.trim(),
        password: form.password,
        folders: [form.folder.trim() || 'INBOX'],
        archive_folder: form.archive.trim() || null,
      },
    })
    form.password = ''
  }
}
</script>

<template>
  <form class="settings-form" data-testid="add-mail-source" @submit.prevent="submit">
    <h2>Add a mail source</h2>
    <label>
      Type
      <select v-model="provider" name="provider">
        <option value="gmail">Gmail (Google sign-in)</option>
        <option value="imap">IMAP (any provider, app password)</option>
      </select>
    </label>
    <label>Name <input v-model="form.name" name="name" required /></label>

    <template v-if="provider === 'gmail'">
      <p v-if="needsScope" class="settings-note">
        Your Google account has not been given Gmail access yet.
        <template v-if="googleEnabled">
          <a
            class="button"
            :href="mailApi.googleStartUrl('gmail_read', RETURN_TO)"
            data-testid="connect-gmail-read"
            >Connect Gmail (read only)</a
          >
          <a class="button" :href="mailApi.googleStartUrl('gmail_modify', RETURN_TO)"
            >Connect Gmail (read and archive)</a
          >
        </template>
        <template v-else>Google sign-in is not configured on this server.</template>
      </p>
      <label v-else>
        Google account
        <select v-model="form.account" name="account" required>
          <option value="" disabled>Choose…</option>
          <option v-for="a in gmailAccounts" :key="a.id" :value="a.id">{{ a.email }}</option>
        </select>
      </label>
    </template>
    <template v-else>
      <label>
        Email address (user name)
        <input v-model="form.username" name="username" required @change="onUsername" />
      </label>
      <label
        >Server <input v-model="form.host" name="host" placeholder="imap.example.com" required
      /></label>
      <label
        >Port <input v-model.number="form.port" name="port" type="number" min="1" max="65535"
      /></label>
      <label>
        App password
        <input
          v-model="form.password"
          name="password"
          type="password"
          autocomplete="off"
          required
        />
      </label>
      <label>Folder to read <input v-model="form.folder" name="folder" /></label>
      <label>
        Archive folder (used when archiving from the app)
        <input v-model="form.archive" name="archive" />
      </label>
      <div>
        <button type="button" @click="test">Test connection</button>
        <span
          v-if="testNote"
          :class="testOk ? 'settings-ok' : 'settings-error'"
          role="status"
          data-testid="imap-test"
          >{{ testNote }}</span
        >
      </div>
    </template>

    <fieldset>
      <legend>Owners (whose mail this is)</legend>
      <label v-for="m in adults" :key="m.id" class="inline">
        <input v-model="form.owners" type="checkbox" :value="m.id" />
        {{ m.display_name }}
      </label>
    </fieldset>
    <label class="inline">
      <input v-model="form.readOnly" type="checkbox" name="read_only" />
      Read only (never archive, label or mark mail read)
    </label>
    <label>
      Days of history to import
      <input v-model.number="form.backfill" name="backfill" type="number" min="1" max="365" />
    </label>
    <label>
      Filter (optional, Gmail search syntax)
      <input v-model="form.query" name="query" />
    </label>
    <p v-if="error" class="settings-error" role="alert">{{ error }}</p>
    <button type="submit" :disabled="needsScope">Add source</button>
  </form>
</template>

<style scoped>
.button {
  margin-left: var(--space-2);
  padding: 0.1rem 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  text-decoration: none;
}
</style>
