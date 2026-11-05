<script setup lang="ts">
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
import ResponsiveItems from '../app/ResponsiveItems.vue'
import SettingsPage from '../app/SettingsPage.vue'
import {
  UiButton,
  UiCard,
  UiChip,
  UiDialog,
  UiEmptyState,
  UiErrorState,
  UiFab,
  UiSheet,
  UiSkeleton,
  UiTextField,
  useSnackbar,
} from '../ui'

const snackbar = useSnackbar()
const devices = ref<DeviceDto[]>([])
const loading = ref(true)
const pairing = ref(false)
const revoking = ref<DeviceDto | null>(null)
const name = ref('')
const paired = ref<PairDeviceResp | null>(null)
const qr = ref('')
const lanQr = ref('')
const error = ref('')

async function load() {
  try {
    devices.value = await api.devices()
    loadError.value = ''
  } catch (e) {
    loadError.value = e instanceof ApiError ? e.message : 'Could not load devices.'
  } finally {
    loading.value = false
  }
}
const loadError = ref('')

function openPair() {
  paired.value = null
  qr.value = ''
  lanQr.value = ''
  error.value = ''
  pairing.value = true
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

async function revoke() {
  const d = revoking.value
  revoking.value = null
  if (!d) return
  try {
    await api.revokeDevice(d.id)
    snackbar.show({ text: `${d.name} revoked` })
    await load()
  } catch (e) {
    snackbar.show({ text: e instanceof ApiError ? e.message : 'Revoke failed.' })
  }
}

const lastSeen = (iso: string) =>
  new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
</script>

<template>
  <SettingsPage title="Devices">
    <UiErrorState v-if="loadError" :message="loadError" @retry="load" />
    <UiSkeleton v-else-if="loading" :lines="4" />
    <UiEmptyState
      v-else-if="!devices.length"
      icon="devices"
      title="No devices paired yet"
      text="Pair a wall screen to show the dashboard on it."
    >
      <UiButton icon="add" @click="openPair">Pair a device</UiButton>
    </UiEmptyState>
    <ResponsiveItems v-else :items="devices" class="devices">
      <template #item="{ item: d }">
        <div class="device" data-testid="device">
          <div class="device__head">
            <strong>{{ d.name }}</strong>
            <UiChip v-if="d.revoked" icon="lock">Revoked</UiChip>
            <UiChip v-else>{{ d.kind }}</UiChip>
          </div>
          <p v-if="!d.revoked && d.last_seen" class="muted">
            Last seen {{ lastSeen(d.last_seen) }}
          </p>
          <div v-if="!d.revoked" class="device__actions">
            <UiButton v-if="d.kind === 'dashboard'" variant="tonal" icon="edit" @click="edit(d)">
              {{ S.editLayout }}
            </UiButton>
            <UiButton variant="text" icon="delete" @click="revoking = d">Revoke</UiButton>
          </div>
        </div>
      </template>
    </ResponsiveItems>

    <UiCard v-if="editing && draft && preview" data-testid="device-layout">
      <template #header>{{ S.layout }}: {{ editing.name }}</template>
      <p v-if="notice" role="status">{{ notice }}</p>
      <p v-if="error" role="alert" class="error">{{ error }}</p>
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
      <template #actions>
        <UiButton variant="text" @click="closeEditor">{{ S.closeEditor }}</UiButton>
      </template>
    </UiCard>
    <p v-else-if="error" role="alert" class="error">{{ error }}</p>

    <div class="fab"><UiFab icon="add" label="Pair a device" extended @click="openPair" /></div>

    <UiSheet v-model="pairing" title="Pair a device">
      <form v-if="!paired" class="pair" @submit.prevent="pair">
        <UiTextField v-model="name" label="Device name" name="device_name" required />
        <p v-if="error" role="alert" class="error">{{ error }}</p>
        <UiButton type="submit">Pair</UiButton>
      </form>
      <div v-else class="paired" data-testid="paired">
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
      <template v-if="paired" #actions>
        <UiButton @click="pairing = false">Done</UiButton>
      </template>
    </UiSheet>

    <UiDialog
      :model-value="!!revoking"
      :title="`Revoke ${revoking?.name ?? ''}?`"
      @update:model-value="!$event && (revoking = null)"
    >
      <p>The screen stops working until you pair it again.</p>
      <template #actions>
        <UiButton variant="text" @click="revoking = null">Cancel</UiButton>
        <UiButton @click="revoke">Revoke</UiButton>
      </template>
    </UiDialog>
  </SettingsPage>
</template>

<style scoped>
.fab {
  position: fixed;
  right: calc(var(--ui-space-4) + env(safe-area-inset-right));
  bottom: calc(var(--ui-bottom-offset, 0px) + var(--ui-space-4) + env(safe-area-inset-bottom));
  z-index: 15;
}
.device {
  display: grid;
  gap: var(--ui-space-2);
}
.device__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ui-space-2);
}
.device__actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.muted {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.error {
  color: var(--ui-error);
  margin: 0;
}
.pair,
.paired {
  display: grid;
  gap: var(--ui-space-3);
}
code {
  word-break: break-all;
}
.split {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr));
  gap: var(--ui-space-5);
  align-items: start;
}
</style>
