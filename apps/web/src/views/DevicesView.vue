<script setup lang="ts">
import QRCode from 'qrcode'
import { onMounted, ref } from 'vue'
import { api, type DeviceDto, type PairDeviceResp } from '../api/client'
import { ApiError } from '../api/http'

const devices = ref<DeviceDto[]>([])
const name = ref('')
const paired = ref<PairDeviceResp | null>(null)
const qr = ref('')
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
    name.value = ''
    await load()
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Pairing failed.'
  }
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
    <h1>Devices</h1>
    <p v-if="error" class="error" role="alert">{{ error }}</p>

    <form @submit.prevent="pair">
      <label>Device name <input v-model="name" name="device_name" required /></label>
      <button type="submit">Pair a device</button>
    </form>

    <div v-if="paired" class="paired" data-testid="paired">
      <p>
        Open this URL on the screen (shown once):
        <code data-testid="pair-url">{{ paired.pair_url }}</code>
      </p>
      <img v-if="qr" :src="qr" alt="QR code for the pair URL" width="240" height="240" />
    </div>

    <h2>Paired devices</h2>
    <ul>
      <li v-for="d in devices" :key="d.id">
        <strong>{{ d.name }}</strong> ({{ d.kind }})
        <span v-if="d.revoked"> revoked</span>
        <span v-else-if="d.last_seen"> last seen {{ d.last_seen }}</span>
        <button v-if="!d.revoked" @click="revoke(d)">Revoke</button>
      </li>
    </ul>
  </section>
</template>

<style scoped>
form {
  display: flex;
  gap: 0.75rem;
  align-items: end;
}
label {
  display: grid;
  gap: 0.15rem;
}
code {
  word-break: break-all;
}
.error {
  color: #c0392b;
}
li {
  margin: 0.4rem 0;
}
</style>
