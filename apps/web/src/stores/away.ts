import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type MemberAwayView, type NewAway } from '../api/client'

/** Absences of every loaded member, by member id. */
export const useAwayStore = defineStore('away', () => {
  const byMember = ref<Record<string, MemberAwayView[]>>({})
  const error = ref<string | null>(null)

  async function load(memberIds: string[]): Promise<void> {
    try {
      const all = await Promise.all(memberIds.map((id) => api.away(id)))
      const next: Record<string, MemberAwayView[]> = {}
      memberIds.forEach((id, i) => (next[id] = all[i]!))
      byMember.value = next
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  async function add(member: string, b: NewAway): Promise<void> {
    await api.addAway(member, b)
    byMember.value[member] = await api.away(member)
  }

  async function remove(member: string, id: string): Promise<void> {
    await api.deleteAway(id)
    byMember.value[member] = await api.away(member)
  }

  async function invalidate(kind: string): Promise<void> {
    const ids = Object.keys(byMember.value)
    if (ids.length && (kind === 'tasks.updated' || kind === 'resync')) await load(ids)
  }

  return { byMember, error, load, add, remove, invalidate }
})
