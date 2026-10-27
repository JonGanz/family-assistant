<script setup lang="ts">
import { computed, ref } from 'vue'
import { api, type ImportReport } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { downloadText } from '../../lib/download'
import { describeError } from '../shared'
import { summarise } from './importReport'

withDefaults(defineProps<{ exportable?: boolean }>(), { exportable: true })
const emit = defineEmits<{ applied: [] }>()

const toml = ref('')
const fileName = ref('')
const report = ref<ImportReport | null>(null)
const applied = ref(false)
const error = ref('')
const busy = ref(false)
const rows = computed(() => (report.value ? summarise(report.value) : []))

async function exportFile() {
  error.value = ''
  try {
    downloadText('household.toml', await api.exportConfig())
  } catch (e) {
    error.value = describeError(e, 'Could not export the household file.')
  }
}

async function pick(ev: Event) {
  const file = (ev.target as HTMLInputElement).files?.[0]
  report.value = null
  applied.value = false
  error.value = ''
  if (!file) return
  if (file.size > 1024 * 1024) {
    error.value = 'That file is larger than 1 MiB.'
    return
  }
  fileName.value = file.name
  toml.value = await file.text()
  await run(true)
}

async function run(dry: boolean) {
  error.value = ''
  busy.value = true
  try {
    report.value = await api.importConfig(toml.value, dry)
    if (!dry) {
      applied.value = true
      emit('applied')
    }
  } catch (e) {
    report.value = null
    error.value = describeError(e, 'The file could not be read.')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="household-file" data-testid="household-file">
    <div v-if="exportable" class="row">
      <button type="button" @click="exportFile">Export household file</button>
      <span class="muted">TOML, without secrets.</span>
    </div>
    <label class="field">
      Import a household file
      <input type="file" name="household_file" accept=".toml,text/plain" @change="pick" />
    </label>
    <ErrorBanner v-if="error" :message="error" />
    <div v-if="report" data-testid="import-preview">
      <p>
        <strong>{{ applied ? 'Imported' : 'Preview' }}</strong> of {{ fileName }}:
      </p>
      <ul class="card-list">
        <li v-for="r in rows" :key="r.label">
          {{ r.label }}: {{ r.created }} new, {{ r.updated }} changed,
          {{ r.unchanged }} unchanged<span v-if="r.skipped">, {{ r.skipped }} skipped</span>
        </li>
      </ul>
      <button
        v-if="!applied"
        type="button"
        class="btn-primary"
        :disabled="busy"
        @click="run(false)"
      >
        Apply
      </button>
    </div>
  </div>
</template>
