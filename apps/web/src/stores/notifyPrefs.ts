import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type NotifyPrefPatch, type NotifyPrefView } from '../api/client'
import { isConflict } from '../api/http'

export type TestResult = { ok: true } | { ok: false; message: string }

export const useNotifyPrefsStore = defineStore('notifyPrefs', () => {
  const prefs = ref<NotifyPrefView | null>(null)
  const member = ref<string | null>(null)
  const error = ref<string | null>(null)
  const conflict = ref(false)

  async function load(id: string): Promise<void> {
    member.value = id
    try {
      prefs.value = await api.notifyPrefs(id)
      error.value = null
      conflict.value = false
    } catch (e) {
      prefs.value = null
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  /** Returns false on a 409. */
  async function save(patch: NotifyPrefPatch): Promise<boolean> {
    const p = prefs.value
    if (!p) return false
    try {
      prefs.value = await api.putNotifyPrefs(p.member, p.version, patch)
      conflict.value = false
      return true
    } catch (e) {
      if (isConflict(e)) {
        conflict.value = true
        return false
      }
      throw e
    }
  }

  async function test(): Promise<TestResult> {
    if (!member.value) return { ok: false, message: 'No member selected.' }
    try {
      await api.testNotify(member.value)
      return { ok: true }
    } catch (e) {
      const status = (e as { status?: number }).status
      if (status === 503)
        return { ok: false, message: 'Notifications are not configured on this server.' }
      if (status === 502)
        return { ok: false, message: 'The notification server did not accept the test message.' }
      return { ok: false, message: e instanceof Error ? e.message : 'The test failed.' }
    }
  }

  async function invalidate(kind: string): Promise<void> {
    if (
      member.value &&
      (kind === 'member.updated' || kind === 'household.updated' || kind === 'resync')
    )
      await load(member.value)
  }

  return { prefs, member, error, conflict, load, save, test, invalidate }
})
