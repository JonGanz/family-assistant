import type { components } from '../gen/api'
import { request } from './http'

type S = components['schemas']
export type MailSourceView = S['MailSourceView']
export type MailSourcePatch = S['MailSourcePatch']
export type NewMailSource = S['NewMailSource']
export type ImapSettings = S['ImapSettings']
export type ImapTestRequest = S['ImapTestRequest']
export type ImapTestResult = S['ImapTestResult']
export type MailSourceState = S['MailSourceState']
export type MailProvider = S['MailProvider']
export type MailVisibility = S['MailVisibility']
export type CategoryView = S['CategoryView']
export type CategoryInput = S['CategoryInput']
export type CategoryPatch = S['CategoryPatch']
export type CategoryBehaviour = S['CategoryBehaviour']
export type AutoAction = S['AutoAction']
export type EmailTasksMode = S['EmailTasksMode']
export type MailRuleView = S['MailRuleView']
export type MailRuleInput = S['MailRuleInput']
export type MailRulePatch = S['MailRulePatch']
export type RuleCondition = S['RuleCondition']
export type RuleField = S['RuleField']
export type RuleOp = S['RuleOp']
export type RuleTestResult = S['RuleTestResult']
export type DecisionPointKey = S['DecisionPointKey']
export type SignalView = S['SignalView']
export type OrgView = S['OrgView']
export type OrgInput = S['OrgInput']
export type OrgPatch = S['OrgPatch']
export type OrgKind = S['OrgKind']
export type ContactMode = S['ContactMode']
export type ContactRole = S['ContactRole']
export type OrgContactView = S['OrgContactView']
export type OrgContactInput = S['OrgContactInput']
export type OrgContactPatch = S['OrgContactPatch']
export type EnrollmentView = S['EnrollmentView']
export type EnrollmentInput = S['EnrollmentInput']
export type EnrollmentPatch = S['EnrollmentPatch']
export type InboxView = S['InboxView']
export type InboxItem = S['InboxItem']
export type InboxCounts = S['InboxCounts']
export type InboxKind = 'event' | 'bill' | 'task' | 'contact'
export type EventApproval = S['EventApproval']
export type BillPatch = S['BillPatch']
export type TaskApproval = S['TaskApproval']
export type ContactApproval = S['ContactApproval']
export type EventProposalView = S['EventProposalView']
export type BillView = S['BillView']
export type ContactSuggestionView = S['ContactSuggestionView']
export type Gender = S['Gender']
export type ApprovalMode = S['ApprovalMode']
export type ContentPolicy = S['ContentPolicy']
export type SourceViewExt = S['SourceViewExt']
export type CalendarViewExt = S['CalendarViewExt']
export type SourcePatchExt = S['SourcePatchExt']
export type CalendarPatchExt = S['CalendarPatchExt']
export type MemberDto = S['MemberDto']

/** The member patch fields the M3 editor sends. */
export type MemberM3Patch = Pick<S['PatchMemberReq'], 'aliases' | 'gender' | 'relevance_tags'>

const U = { bearer: false } as const
const ifMatch = (version: number) => ({ 'If-Match': String(version) })
const json = (method: string, body?: unknown, version?: number) => ({
  method,
  body,
  headers: version === undefined ? undefined : ifMatch(version),
  ...U,
})

export const mailApi = {
  sources: () => request<MailSourceView[]>('/api/v1/mail-sources', U),
  createSource: (b: NewMailSource) =>
    request<MailSourceView>('/api/v1/mail-sources', json('POST', b)),
  patchSource: (id: string, version: number, b: MailSourcePatch) =>
    request<MailSourceView>(`/api/v1/mail-sources/${id}`, json('PATCH', b, version)),
  deleteSource: (id: string) => request<void>(`/api/v1/mail-sources/${id}`, json('DELETE')),
  syncSource: (id: string) => request<void>(`/api/v1/mail-sources/${id}/sync`, json('POST')),
  testImap: (b: ImapTestRequest) =>
    request<ImapTestResult>('/api/v1/mail-sources/imap/test', json('POST', b)),
  /** Full-page navigation target (302 to Google). */
  googleStartUrl: (
    purpose: 'calendar' | 'gmail_read' | 'gmail_modify' | 'calendar_write',
    returnTo: string,
  ) => `/api/v1/oauth/google/start?${new URLSearchParams({ purpose, return_to: returnTo })}`,

  categories: () => request<CategoryView[]>('/api/v1/categories', U),
  createCategory: (b: CategoryInput) =>
    request<CategoryView>('/api/v1/categories', json('POST', b)),
  patchCategory: (id: string, version: number, b: CategoryPatch) =>
    request<CategoryView>(`/api/v1/categories/${id}`, json('PATCH', b, version)),

  rules: () => request<MailRuleView[]>('/api/v1/mail-rules', U),
  createRule: (b: MailRuleInput) => request<MailRuleView>('/api/v1/mail-rules', json('POST', b)),
  patchRule: (id: string, version: number, b: MailRulePatch) =>
    request<MailRuleView>(`/api/v1/mail-rules/${id}`, json('PATCH', b, version)),
  deleteRule: (id: string) => request<void>(`/api/v1/mail-rules/${id}`, json('DELETE')),
  testRule: (rule: MailRuleInput, limit?: number) =>
    request<RuleTestResult>(
      '/api/v1/mail-rules/test',
      json('POST', { rule, limit: limit ?? null }),
    ),

  signals: () => request<SignalView[]>('/api/v1/mail-signals', U),
  patchSignal: (key: string, enabled: boolean) =>
    request<SignalView>(
      `/api/v1/mail-signals/${encodeURIComponent(key)}`,
      json('PATCH', { enabled }),
    ),

  orgs: () => request<OrgView[]>('/api/v1/orgs', U),
  createOrg: (b: OrgInput) => request<OrgView>('/api/v1/orgs', json('POST', b)),
  patchOrg: (id: string, version: number, b: OrgPatch) =>
    request<OrgView>(`/api/v1/orgs/${id}`, json('PATCH', b, version)),
  deleteOrg: (id: string) => request<void>(`/api/v1/orgs/${id}`, json('DELETE')),
  createContact: (org: string, b: OrgContactInput) =>
    request<OrgContactView>(`/api/v1/orgs/${org}/contacts`, json('POST', b)),
  patchContact: (id: string, version: number, b: OrgContactPatch) =>
    request<OrgContactView>(`/api/v1/org-contacts/${id}`, json('PATCH', b, version)),
  deleteContact: (id: string) => request<void>(`/api/v1/org-contacts/${id}`, json('DELETE')),
  enrollments: (member: string) =>
    request<EnrollmentView[]>(`/api/v1/members/${member}/enrollments`, U),
  createEnrollment: (member: string, b: EnrollmentInput) =>
    request<EnrollmentView>(`/api/v1/members/${member}/enrollments`, json('POST', b)),
  patchEnrollment: (id: string, version: number, b: EnrollmentPatch) =>
    request<EnrollmentView>(`/api/v1/enrollments/${id}`, json('PATCH', b, version)),
  deleteEnrollment: (id: string) => request<void>(`/api/v1/enrollments/${id}`, json('DELETE')),

  /** Members editor: the M3 fields on top of the generated patch type. */
  patchMemberM3: (id: string, version: number, b: MemberM3Patch) =>
    request<MemberDto>(`/api/v1/members/${id}`, json('PATCH', b, version)),

  calendarSources: () => request<SourceViewExt[]>('/api/v1/calendar-sources', U),
  patchCalendarSource: (id: string, version: number, b: SourcePatchExt) =>
    request<SourceViewExt>(`/api/v1/calendar-sources/${id}`, json('PATCH', b, version)),
  calendars: () => request<CalendarViewExt[]>('/api/v1/calendars', U),
  patchCalendar: (id: string, version: number, b: CalendarPatchExt) =>
    request<CalendarViewExt>(`/api/v1/calendars/${id}`, json('PATCH', b, version)),
}

/** Inbox calls made with the kiosk's device bearer (the elevated device). */
export const inboxApi = {
  list: (kind?: InboxKind) => request<InboxView>(`/api/v1/inbox${kind ? `?kind=${kind}` : ''}`),
  approve: (kind: InboxKind, id: string, body?: unknown) =>
    request<void>(`/api/v1/inbox/${kind}/${id}/approve`, { method: 'POST', body: body ?? {} }),
  reject: (kind: InboxKind, id: string) =>
    request<void>(`/api/v1/inbox/${kind}/${id}/reject`, { method: 'POST', body: {} }),
}
