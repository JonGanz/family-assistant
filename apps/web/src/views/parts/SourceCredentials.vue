<script setup lang="ts">
import { reactive, ref } from 'vue'
import type { SourceView } from '../../api/client'
import { useSourcesStore } from '../../stores/sources'
import { UiCard, UiTextField } from '../../ui'
import { describeError } from '../shared'
import SheetForm from './settings-mail/SheetForm.vue'

const props = defineProps<{ source: SourceView; modelValue: boolean }>()
const emit = defineEmits<{ done: []; 'update:modelValue': [v: boolean] }>()
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
  <SheetForm
    :model-value="modelValue"
    :title="`Change ${source.name}`"
    save-label="Save and sync"
    @update:model-value="emit('update:modelValue', $event)"
    @save="save"
  >
    <p>Leave a field empty to keep its current value.</p>
    <UiTextField
      v-model="form.url"
      :label="source.kind === 'ics' ? 'New calendar link' : 'New server URL'"
      name="new_url"
      type="url"
    />
    <template v-if="source.kind === 'caldav'">
      <UiTextField v-model="form.username" label="Username" name="new_user" />
      <UiTextField
        v-model="form.password"
        label="Password"
        name="new_password"
        type="password"
        autocomplete="new-password"
      />
    </template>
    <UiCard v-if="error" variant="outlined" role="alert">{{ error }}</UiCard>
  </SheetForm>
</template>
