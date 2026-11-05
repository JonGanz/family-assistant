<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { OAuthAccountView } from '../../../api/client'
import { mailApi, type NewMailSource } from '../../../api/mailSettings'
import type { Member } from '../../../api/client'
import { GOOGLE_NEEDS_HTTPS, isSecure } from '../../../lib/secure'
import { UiButton, UiCard, UiCheckbox, UiSelect, UiTextField } from '../../../ui'
import { describeError } from '../../shared'
import { toggleIn } from './toggle'

const props = defineProps<{
  members: Member[]
  accounts: OAuthAccountView[]
  googleEnabled: boolean
  meId?: string
}>()
const emit = defineEmits<{ create: [b: NewMailSource] }>()

const RETURN_TO = '/settings/mail'
const secure = isSecure()
const provider = ref<'gmail' | 'imap'>(secure ? 'gmail' : 'imap')
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
const typeOptions = computed(() => [
  {
    value: 'gmail',
    label: secure ? 'Gmail (Google sign-in)' : `Gmail (${GOOGLE_NEEDS_HTTPS})`,
    disabled: !secure,
  },
  { value: 'imap', label: 'IMAP (any provider, app password)' },
])
const accountOptions = computed(() => [
  { value: '', label: 'Choose…', disabled: true },
  ...gmailAccounts.value.map((a) => ({ value: a.id, label: a.email })),
])
const go = (url: string) => (window.location.href = url)
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
  if (!form.name.trim()) {
    error.value = 'Give the source a name.'
    return
  }
  if (provider.value === 'imap' && !(form.host.trim() && form.username.trim() && form.password)) {
    error.value = 'Enter the server, the email address and the app password.'
    return
  }
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
  <form class="add" data-testid="add-mail-source" @submit.prevent="submit">
    <UiSelect
      :model-value="provider"
      label="Type"
      name="provider"
      :options="typeOptions"
      @update:model-value="provider = $event as 'gmail' | 'imap'"
    />
    <UiTextField v-model="form.name" label="Name" name="name" />

    <template v-if="provider === 'gmail'">
      <template v-if="needsScope">
        <p>Your Google account has not been given Gmail access yet.</p>
        <template v-if="googleEnabled">
          <UiButton
            variant="tonal"
            data-testid="connect-gmail-read"
            @click="go(mailApi.googleStartUrl('gmail_read', RETURN_TO))"
            >Connect Gmail (read only)</UiButton
          >
          <UiButton variant="tonal" @click="go(mailApi.googleStartUrl('gmail_modify', RETURN_TO))"
            >Connect Gmail (read and archive)</UiButton
          >
        </template>
        <p v-else>Google sign-in is not configured on this server.</p>
      </template>
      <UiSelect v-else v-model="form.account" label="Google account" :options="accountOptions" />
    </template>
    <template v-else>
      <UiTextField
        v-model="form.username"
        label="Email address (user name)"
        name="username"
        @change="onUsername"
      />
      <UiTextField v-model="form.host" label="Server" name="host" placeholder="imap.example.com" />
      <UiTextField
        :model-value="String(form.port)"
        label="Port"
        name="port"
        type="number"
        min="1"
        max="65535"
        @update:model-value="form.port = Number($event)"
      />
      <UiTextField
        v-model="form.password"
        label="App password"
        name="password"
        type="password"
        autocomplete="off"
      />
      <UiTextField v-model="form.folder" label="Folder to read" name="folder" />
      <UiTextField
        v-model="form.archive"
        label="Archive folder (used when archiving from the app)"
        name="archive"
      />
      <div class="add__row">
        <UiButton variant="tonal" @click="test">Test connection</UiButton>
        <span v-if="testNote" role="status" data-testid="imap-test">{{ testNote }}</span>
      </div>
    </template>

    <fieldset class="add__owners">
      <legend>Owners (whose mail this is)</legend>
      <UiCheckbox
        v-for="m in adults"
        :key="m.id"
        :model-value="form.owners.includes(m.id)"
        :label="m.display_name"
        @update:model-value="toggleIn(form.owners, m.id, $event)"
      />
    </fieldset>
    <UiCheckbox
      v-model="form.readOnly"
      label="Read only (never archive, label or mark mail read)"
    />
    <UiTextField
      :model-value="String(form.backfill)"
      label="Days of history to import"
      name="backfill"
      type="number"
      min="1"
      max="365"
      @update:model-value="form.backfill = Number($event)"
    />
    <UiTextField v-model="form.query" label="Filter (optional, Gmail search syntax)" name="query" />
    <UiCard v-if="error" variant="outlined" role="alert" data-testid="add-error">{{
      error
    }}</UiCard>
    <UiButton type="submit" :disabled="needsScope">Add source</UiButton>
  </form>
</template>

<style scoped>
.add {
  display: grid;
  gap: var(--ui-space-3);
}
.add__row {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.add__owners {
  display: grid;
  gap: var(--ui-space-1);
  border: 0;
  padding: 0;
  margin: 0;
}
</style>
