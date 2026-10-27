<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { copyText } from '../../lib/copy'
import { downloadText } from '../../lib/download'
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
      status === 401 ? 'That password is wrong.' : describeError(e, 'Could not show the key.')
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
      The master key encrypts the passwords and tokens this server stores. If the server's disk is
      lost and you have no copy of the key, those secrets cannot be recovered. Keep a copy somewhere
      safe, such as a password manager.
    </p>
    <form v-if="!key" class="form" @submit.prevent="reveal">
      <label class="field">
        Your password, to show the key
        <input
          v-model="password"
          name="master_key_password"
          type="password"
          autocomplete="current-password"
          required
        />
      </label>
      <ErrorBanner v-if="error" :message="error" />
      <button type="submit" class="btn-primary" :disabled="busy">Show the key</button>
    </form>
    <div v-else class="form">
      <label class="field">
        Master key
        <input
          :value="key"
          name="master_key"
          readonly
          data-testid="master-key"
          @focus="($event.target as HTMLInputElement).select()"
        />
      </label>
      <div class="row">
        <button type="button" @click="copy">Copy</button>
        <button type="button" @click="save">Download</button>
      </div>
      <p v-if="copied === 'yes'" class="text-success" role="status">Copied.</p>
      <p v-if="copied === 'no'" class="muted" role="status">
        Copying is not available here. Select the key above and press Ctrl+C.
      </p>
      <label class="field-inline">
        <input v-model="stored" type="checkbox" name="key_stored" />
        I stored it somewhere safe
      </label>
      <ErrorBanner v-if="error" :message="error" />
      <button type="button" class="btn-primary" :disabled="!stored" @click="confirm">
        Confirm backup
      </button>
    </div>
  </div>
</template>
