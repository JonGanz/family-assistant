import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type CreateMemberReq, type Member, type PatchMemberReq } from '../api/client'
import { ApiError } from '../api/http'

export const useMembersStore = defineStore('members', () => {
  const items = ref<Member[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  /** Set when the last save hit a 409 version conflict. */
  const conflictId = ref<string | null>(null)

  async function load(device = false): Promise<void> {
    loading.value = true
    error.value = null
    try {
      items.value = await api.members(device)
      conflictId.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  async function create(req: CreateMemberReq): Promise<void> {
    await api.createMember(req)
    await load()
  }

  /** Returns true on success, false on a 409 conflict (state kept so the UI can offer reload). */
  async function update(m: Member, patch: PatchMemberReq): Promise<boolean> {
    try {
      await api.patchMember(m.id, m.version ?? 0, patch)
      conflictId.value = null
      await load()
      return true
    } catch (e) {
      if (e instanceof ApiError && e.status === 409) {
        conflictId.value = m.id
        return false
      }
      throw e
    }
  }

  return { items, loading, error, conflictId, load, create, update }
})
