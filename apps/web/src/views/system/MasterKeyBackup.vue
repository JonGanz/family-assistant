<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../../api/client'
import { copyText } from '../../lib/copy'
import { downloadText } from '../../lib/download'
import { UiButton, UiCard, UiCheckbox, UiTextField } from '../../ui'
import { describeError } from '../shared'

const emit = defineEmits<{ confirmed: [] }>()

const password = ref('')
const key = ref('')
const stored = ref(false)
const copied = ref<'yes' | 'no' | ''>('')
const error = ref('')
const busy = ref(false)

async function reveal() {
  error.value = ''
  busy.value = true
  try {
    key.value = (await api.masterKey(password.value)).key_base64
    password.value = ''
  } catch (e) {
    const status = (e as { status?: number }).status
    error.value =
      status === 401
        ? 'That is not your login password. Use the one you set for your own account, not a PIN.'
        : describeError(e, 'Could not show the key.')
  } finally {
    busy.value = false
  }
}

async function copy() {
  copied.value = (await copyText(key.value)) ? 'yes' : 'no'
}

const save = () => downloadText('family-assistant-master-key.txt', `${key.value}\n`)

async function confirm() {
  error.value = ''
  try {
    await api.confirmMasterKey()
    key.value = ''
    emit('confirmed')
  } catch (e) {
    error.value = describeError(e, 'Could not record the confirmation.')
  }
}
</script>

<template>
  <div class="master-key" data-testid="master-key-backup">
    <p>
      The master key encrypts the passwords and tokens this server stores. It was created
      automatically when the server first started, so there is nothing for you to choose. If the
      server's disk is lost and you have no copy of the key, those secrets cannot be recovered. Keep
      a copy somewhere safe, such as a password manager.
    </p>
    <template v-if="!key">
      <p>
        To show the key, enter your own login password, the one you set for your account in the
        setup. This only confirms that it is you. It is not a password for the key.
      </p>
      <form class="stack" @submit.prevent="reveal">
        <UiTextField
          v-model="password"
          label="Your login password"
          name="master_key_password"
          type="password"
          autocomplete="current-password"
          required
        />
        <UiCard v-if="error" variant="outlined" role="alert">{{ error }}</UiCard>
        <div><UiButton type="submit" :loading="busy">Show the key</UiButton></div>
      </form>
    </template>
    <div v-else class="stack">
      <UiTextField
        :model-value="key"
        label="Master key"
        name="master_key"
        readonly
        data-testid="master-key"
        @focus="($event.target as HTMLInputElement).select()"
      />
      <div class="row">
        <UiButton variant="tonal" @click="copy">Copy</UiButton>
        <UiButton variant="tonal" @click="save">Download</UiButton>
      </div>
      <p v-if="copied === 'yes'" role="status">Copied.</p>
      <p v-if="copied === 'no'" role="status">
        Copying is not available here. Select the key above and press Ctrl+C.
      </p>
      <UiCheckbox v-model="stored" label="I stored it somewhere safe" />
      <UiCard v-if="error" variant="outlined" role="alert">{{ error }}</UiCard>
      <div><UiButton :disabled="!stored" @click="confirm">Confirm backup</UiButton></div>
    </div>
  </div>
</template>

<style scoped>
.master-key,
.stack {
  display: grid;
  gap: var(--ui-space-3);
}
.row {
  display: flex;
  gap: var(--ui-space-2);
}
p {
  margin: 0;
}
</style>
