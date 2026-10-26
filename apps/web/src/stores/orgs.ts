import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  mailApi,
  type EnrollmentInput,
  type EnrollmentPatch,
  type EnrollmentView,
  type OrgContactInput,
  type OrgContactPatch,
  type OrgContactView,
  type OrgInput,
  type OrgPatch,
  type OrgView,
} from '../api/mailSettings'
import { isConflict } from '../api/http'

export const useOrgsStore = defineStore('orgs', () => {
  const orgs = ref<OrgView[]>([])
  /** Enrollments by member id. */
  const enrollments = ref<Record<string, EnrollmentView[]>>({})
  const error = ref<string | null>(null)
  const conflict = ref(false)
  const loaded = ref(false)

  async function load(): Promise<void> {
    try {
      orgs.value = await mailApi.orgs()
      error.value = null
      conflict.value = false
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  async function loadEnrollments(member: string): Promise<void> {
    enrollments.value = { ...enrollments.value, [member]: await mailApi.enrollments(member) }
  }

  async function guarded(
    fn: () => Promise<unknown>,
    reload: () => Promise<void>,
  ): Promise<boolean> {
    try {
      await fn()
      conflict.value = false
      await reload()
      return true
    } catch (e) {
      if (isConflict(e)) {
        conflict.value = true
        return false
      }
      throw e
    }
  }

  const createOrg = (b: OrgInput) => guarded(() => mailApi.createOrg(b), load)
  const patchOrg = (o: OrgView, p: OrgPatch) =>
    guarded(() => mailApi.patchOrg(o.id, o.version, p), load)
  const removeOrg = (id: string) => guarded(() => mailApi.deleteOrg(id), load)

  const addContact = (org: string, b: OrgContactInput) =>
    guarded(() => mailApi.createContact(org, b), load)
  const patchContact = (c: OrgContactView, p: OrgContactPatch) =>
    guarded(() => mailApi.patchContact(c.id, c.version, p), load)
  const removeContact = (id: string) => guarded(() => mailApi.deleteContact(id), load)

  const addEnrollment = (member: string, b: EnrollmentInput) =>
    guarded(
      () => mailApi.createEnrollment(member, b),
      () => loadEnrollments(member),
    )
  const patchEnrollment = (e: EnrollmentView, p: EnrollmentPatch) =>
    guarded(
      () => mailApi.patchEnrollment(e.id, e.version, p),
      () => loadEnrollments(e.member),
    )
  const removeEnrollment = (e: EnrollmentView) =>
    guarded(
      () => mailApi.deleteEnrollment(e.id),
      () => loadEnrollments(e.member),
    )

  return {
    orgs,
    enrollments,
    error,
    conflict,
    loaded,
    load,
    loadEnrollments,
    createOrg,
    patchOrg,
    removeOrg,
    addContact,
    patchContact,
    removeContact,
    addEnrollment,
    patchEnrollment,
    removeEnrollment,
  }
})
