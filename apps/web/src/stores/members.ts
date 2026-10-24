import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type CreateMemberReq, type Member, type PatchMemberReq } from '../api/client'
import { isConflict } from '../api/http'
import type { LiveEventKind } from '../composables/useLiveStream'

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
      if (isConflict(e)) {
        conflictId.value = m.id
        return false
      }
      throw e
    }
  }

  const AVATAR_MAX = 512 * 1024
  const AVATAR_TYPES = ['image/png', 'image/jpeg', 'image/webp']

  /** Client-side pre-check; the server re-validates (413 / 415). Returns a message or null. */
  function checkAvatar(file: Blob): string | null {
    if (file.size > AVATAR_MAX) return 'That picture is larger than 512 KB. Pick a smaller one.'
    if (!AVATAR_TYPES.includes(file.type)) return 'Use a PNG, JPEG or WebP picture.'
    return null
  }

  async function uploadAvatar(id: string, file: Blob): Promise<void> {
    await api.putAvatar(id, file)
    await load()
  }

  async function removeAvatar(id: string): Promise<void> {
    await api.deleteAvatar(id)
    await load()
  }

  async function invalidate(kind: LiveEventKind): Promise<void> {
    if (items.value.length === 0 && !loading.value) return
    if (kind === 'member.updated' || kind === 'resync') await load()
  }

  return {
    items,
    loading,
    error,
    conflictId,
    load,
    create,
    update,
    checkAvatar,
    uploadAvatar,
    removeAvatar,
    invalidate,
  }
})
