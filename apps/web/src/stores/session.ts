import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api, type HouseholdDto, type MeResponse } from '../api/client'
import { ApiError } from '../api/http'

export const useSessionStore = defineStore('session', () => {
  const me = ref<MeResponse | null>(null)
  const loaded = ref(false)

  const kind = computed(() => me.value?.actor.kind ?? null)
  const isUser = computed(() => kind.value === 'user')
  const household = computed<HouseholdDto | null>(() => me.value?.household ?? null)
  const member = computed(
    () => (me.value?.actor.member ?? null) as { id: string; role: string } | null,
  )
  const mustChangePassword = computed(
    () =>
      isUser.value &&
      (me.value as { must_change_password?: boolean } | null)?.must_change_password === true,
  )
  const isAdult = computed(() => isUser.value && member.value?.role === 'adult')

  async function load(force = false): Promise<void> {
    if (loaded.value && !force) return
    try {
      me.value = await api.me()
    } catch (e) {
      if (!(e instanceof ApiError) || e.status !== 401) throw e
      me.value = null
    }
    loaded.value = true
  }

  async function login(username: string, password: string): Promise<void> {
    await api.login({ username, password })
    await load(true)
  }

  async function logout(): Promise<void> {
    try {
      await api.logout()
    } finally {
      me.value = null
      loaded.value = true
      // The service worker's grocery cache (secure context only, T-08) must not outlive the session on a shared phone.
      try {
        if (typeof caches !== 'undefined') await caches.delete('fa-grocery')
      } catch {
        // No Cache Storage (plain HTTP): nothing was cached.
      }
      try {
        localStorage.removeItem('fa.grocery.queue')
      } catch {
        // Storage blocked: nothing was queued.
      }
    }
  }

  return {
    me,
    loaded,
    kind,
    isUser,
    isAdult,
    household,
    member,
    mustChangePassword,
    load,
    login,
    logout,
  }
})
