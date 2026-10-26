<script setup lang="ts">
import ErrorBanner from '../components/ErrorBanner.vue'
import PageHeader from '../components/PageHeader.vue'
import StateBlock from '../components/StateBlock.vue'
import QRCode from 'qrcode'
import { onMounted, ref } from 'vue'
import { api, type DeviceDto, type PairDeviceResp } from '../api/client'
import { ApiError } from '../api/http'
import DevicePreview from '../dashboard/DevicePreview.vue'
import { clone } from '../dashboard/editor'
import LayoutEditor from '../dashboard/LayoutEditor.vue'
import { loadPreviewData, type PreviewData } from '../dashboard/preview'
import { S } from '../dashboard/strings'
import type { DashboardSettings } from '../dashboard/types'
import './parts/settings.css'

const devices = ref<DeviceDto[]>([])
const name = ref('')
const paired = ref<PairDeviceResp | null>(null)
const qr = ref('')
const lanQr = ref('')
const error = ref('')

async function load() {
  try {
    devices.value = await api.devices()
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Could not load devices.'
  }
}
onMounted(load)

/** The server returns a path-only URL when it has no public base URL; make it absolute. */
function absolute(url: string): string {
  return /^https?:\/\//.test(url) ? url : new URL(url, location.origin).toString()
}

async function pair() {
  error.value = ''
  try {
    const r = await api.pairDevice(name.value)
    paired.value = { ...r, pair_url: absolute(r.pair_url) }
    qr.value = await QRCode.toDataURL(paired.value.pair_url, { margin: 1, width: 240 })
    lanQr.value = r.lan_pair_url
      ? await QRCode.toDataURL(r.lan_pair_url, { margin: 1, width: 240 })
      : ''
    name.value = ''
    await load()
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Pairing failed.'
  }
}

// Layout editor: one device at a time, saved with If-Match.
const editing = ref<{ id: string; name: string; version: number } | null>(null)
const draft = ref<DashboardSettings | null>(null)
const preview = ref<PreviewData | null>(null)
const saving = ref(false)
const notice = ref('')

async function edit(d: DeviceDto) {
  error.value = ''
  notice.value = ''
  try {
    const [view, data] = await Promise.all([
      api.device(d.id),
      preview.value ? Promise.resolve(preview.value) : loadPreviewData(),
    ])
    preview.value = data
    editing.value = { id: view.id, name: view.name, version: view.version }
    draft.value = clone(view.settings)
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : S.saveFailed
  }
}

async function saveLayout() {
  if (!editing.value || !draft.value) return
  saving.value = true
  error.value = ''
  notice.value = ''
  try {
    const r = await api.patchDevice(editing.value.id, editing.value.version, {
      settings: draft.value,
    })
    editing.value = { ...editing.value, version: r.version }
    draft.value = clone(r.settings)
    notice.value = S.saved
  } catch (e) {
    error.value =
      e instanceof ApiError && (e.status === 409 || e.status === 412)
        ? S.conflict
        : e instanceof ApiError
          ? e.message
          : S.saveFailed
  } finally {
    saving.value = false
  }
}

function closeEditor() {
  editing.value = null
  draft.value = null
}

async function revoke(d: DeviceDto) {
  if (!confirm(`Revoke ${d.name}?`)) return
  try {
    await api.revokeDevice(d.id)
    await load()
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Revoke failed.'
  }
}
</script>

<template>
  <section>
    <PageHeader eyebrow="Settings" title="Devices" />
    <ErrorBanner v-if="error" :message="error" />

    <form class="pair" @submit.prevent="pair">
      <label class="field">Device name <input v-model="name" name="device_name" required /></label>
      <button type="submit" class="btn-primary">Pair a device</button>
    </form>

    <div v-if="paired" class="paired" data-testid="paired">
      <p>
        Open this URL on the screen (shown once):
        <code data-testid="pair-url">{{ paired.pair_url }}</code>
      </p>
      <img v-if="qr" :src="qr" alt="QR code for the pair URL" width="240" height="240" />
      <template v-if="paired.lan_pair_url">
        <p>
          On a screen without Tailscale, open this home-network address instead (shown once):
          <code data-testid="lan-pair-url">{{ paired.lan_pair_url }}</code>
        </p>
        <img
          v-if="lanQr"
          :src="lanQr"
          alt="QR code for the home-network pair URL"
          width="240"
          height="240"
        />
      </template>
    </div>

    <h2>Paired devices</h2>
    <StateBlock :empty="!devices.length" empty-text="No devices paired yet." />
    <ul class="card-list">
      <li v-for="d in devices" :key="d.id" class="row">
        <strong>{{ d.name }}</strong> ({{ d.kind }})
        <span v-if="d.revoked"> revoked</span>
        <span v-else-if="d.last_seen"> last seen {{ d.last_seen }}</span>
        <button v-if="!d.revoked && d.kind === 'dashboard'" class="btn-sm" @click="edit(d)">
          {{ S.editLayout }}
        </button>
        <button v-if="!d.revoked" class="btn-danger btn-sm" @click="revoke(d)">Revoke</button>
      </li>
    </ul>

    <section v-if="editing && draft && preview" class="layout" data-testid="device-layout">
      <h2>{{ S.layout }}: {{ editing.name }}</h2>
      <p v-if="notice" role="status">{{ notice }}</p>
      <div class="split">
        <LayoutEditor
          v-model="draft"
          :members="preview.members"
          :lists="preview.task_lists ?? []"
          :saving="saving"
          @save="saveLayout"
        />
        <div>
          <h3>{{ S.preview }}</h3>
          <DevicePreview :data="preview" :settings="draft" />
        </div>
      </div>
      <button type="button" class="btn-ghost" @click="closeEditor">{{ S.closeEditor }}</button>
    </section>
  </section>
</template>

<style scoped>
.pair {
  display: flex;
  gap: var(--space-3);
  align-items: end;
}
code {
  word-break: break-all;
}
.split {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(22rem, 1fr));
  gap: var(--space-5);
  align-items: start;
}
</style>
