<script setup lang="ts">
import { reactive, ref } from 'vue'
import type { SourceView } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { useSourcesStore } from '../../stores/sources'
import { describeError } from '../shared'
import './settings.css'

const props = defineProps<{ source: SourceView }>()
const emit = defineEmits<{ done: [] }>()
const sources = useSourcesStore()
const form = reactive({ url: '', username: '', password: '' })
const error = ref('')

async function save() {
  error.value = ''
  const credentials = {
    url: form.url.trim() || null,
    username: form.username || null,
    password: form.password || null,
  }
  try {
    const ok = await sources.patchSource(props.source, { credentials })
    // Write-only: never keep the secret in the page after sending it.
    Object.assign(form, { url: '', username: '', password: '' })
    if (ok) emit('done')
  } catch (e) {
    error.value = describeError(e, 'Could not save.')
  }
}
</script>

<template>
  <form class="settings-form" autocomplete="off" @submit.prevent="save">
    <p class="settings-note">Leave a field empty to keep its current value.</p>
    <label>
      {{ source.kind === 'ics' ? 'New calendar link' : 'New server URL' }}
      <input v-model="form.url" name="new_url" type="url" autocomplete="off" />
    </label>
    <template v-if="source.kind === 'caldav'">
      <label>Username <input v-model="form.username" name="new_user" autocomplete="off" /></label>
      <label>
        Password
        <input
          v-model="form.password"
          name="new_password"
          type="password"
          autocomplete="new-password"
        />
      </label>
    </template>
    <ErrorBanner v-if="error" :message="error" />
    <button type="submit">Save and sync</button>
  </form>
</template>
