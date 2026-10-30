import type { components, operations } from '../gen/api'
import { http, request } from './http'

type S = components['schemas']
export type LoginResp = S['LoginResp']
export type MeResponse = S['MeResponse']
export type MemberDto = S['MemberDto']
export type MemberPublicDto = S['MemberPublicDto']
export type DeviceDto = S['DeviceDto']
export type PairDeviceResp = S['PairDeviceResp']
export type CreateMemberReq = S['CreateMemberReq']
export type PatchMemberReq = S['PatchMemberReq']
export type HouseholdDto = S['HouseholdDto']
export type HouseholdView = S['HouseholdView']
export type HouseholdPatch = S['HouseholdPatch']
export type EventView = S['EventView']
export type WeatherView = S['WeatherView']
export type AdvisoryView = S['AdvisoryView']
export type SourceView = S['SourceView']
export type NewSource = S['NewSource']
export type SourcePatch = S['SourcePatch']
export type SourceCreated = S['SourceCreated']
export type SourceTestResult = S['SourceTestResult']
export type CalendarView = S['CalendarView']
export type CalendarPatch = S['CalendarPatch']
export type OAuthAccountView = S['OAuthAccountView']
export type WeatherRuleView = S['WeatherRuleView']
export type NewWeatherRule = S['NewWeatherRule']
export type WeatherRulePatch = S['WeatherRulePatch']
export type ValidateRuleResponse = S['ValidateRuleResponse']
export type StatusView = S['StatusView']
export type DashboardView = S['DashboardView']
export type DeviceView = S['DeviceView']
export type DevicePatch = S['DevicePatch']
export type TaskView = S['TaskView']
export type NewTask = S['NewTask']
export type TaskPatch = S['TaskPatch']
export type TaskListView = S['TaskListView']
export type TaskListInput = S['TaskListInput']
export type TaskListPatch = S['TaskListPatch']
export type TaskDefinitionView = S['TaskDefinitionView']
export type TaskDefinitionInput = S['TaskDefinitionInput']
export type TaskDefinitionPatch = S['TaskDefinitionPatch']
export type ValidateDefinitionResponse = S['ValidateDefinitionResponse']
export type RoutineView = S['RoutineView']
export type RoutineInput = S['RoutineInput']
export type RoutinePatch = S['RoutinePatch']
export type RoutineRunView = S['RoutineRunView']
export type MemberAwayView = S['MemberAwayView']
export type NewAway = S['NewAway']
export type NotifyPrefView = S['NotifyPrefView']
export type NotifyPrefPatch = S['NotifyPrefPatch']
export type NotifyTrigger = S['NotifyTrigger']
export type CalendarViewExt = S['CalendarViewExt']
export type SourceViewExt = S['SourceViewExt']
export type MessageSummary = S['MessageSummary']
export type DecisionView = S['DecisionView']
export type LabelInput = S['LabelInput']
export type MessageDetail = S['MessageDetail']
export type SectionView = S['SectionView']
export type MessagePage = S['MessagePage']
export type MailActionInput = S['MailActionInput']
export type MailActionView = S['MailActionView']
export type MailSourceView = S['MailSourceView']
export type CategoryView = S['CategoryView']
export type DecisionPointKey = S['DecisionPointKey']
export type LabelQueueItem = S['LabelQueueItem']
export type MakeTaskInput = S['MakeTaskInput']
export type InboxView = S['InboxView']
export type InboxItem = S['InboxItem']
export type InboxCounts = S['InboxCounts']
export type SetupStatus = S['SetupStatus']
export type NewSetupHousehold = S['NewSetupHousehold']
export type NewSetupOwner = S['NewSetupOwner']
export type SettingView = S['SettingView']
export type SystemSettingsView = S['SystemSettingsView']
export type AboutView = S['AboutView']
export type SystemSettingsPatch = S['SystemSettingsPatch']
export type OwnPasswordChange = S['OwnPasswordChange']
export type ImportReport = S['ImportReport']
export type HouseholdViewExt = S['HouseholdViewExt']
export type PendingApprovals = S['PendingApprovals']
export type InboxKind = 'event' | 'bill' | 'task' | 'contact'
export type EventProposalView = S['EventProposalView']
export type EventApproval = S['EventApproval']
export type TaskApproval = S['TaskApproval']
export type ContactApproval = S['ContactApproval']
export type ContactSuggestionView = S['ContactSuggestionView']
export type BillView = S['BillView']
export type BillPatch = S['BillPatch']

export interface MessageQuery {
  category?: string
  member?: string
  source?: string
  q?: string
  include_ignored?: boolean
  before?: string
  limit?: number
}

export interface TaskQuery {
  view?: 'now' | 'mine' | 'all'
  member?: string
  list?: string
  status?: 'open' | 'done' | 'skipped' | 'suggested' | 'any'
  from?: string
  to?: string
}

/** Adults get every field; children and devices only the public ones. */
export type Member = MemberPublicDto & Partial<Omit<MemberDto, keyof MemberPublicDto>>

type Body<K extends keyof operations> = operations[K] extends {
  requestBody: { content: { 'application/json': infer B } }
}
  ? B
  : never

const U = { bearer: false } as const
const ifMatch = (version: number) => ({ 'If-Match': String(version) })

export const api = {
  login: (b: Body<'login'>) =>
    request<LoginResp>('/api/v1/auth/login', { method: 'POST', body: b, ...U }),
  logout: () => request<void>('/api/v1/auth/logout', { method: 'POST', ...U }),
  /** `device: true` authenticates with the stored device token instead of the session cookie. */
  me: (device = false) => request<MeResponse>('/api/v1/me', { bearer: device }),
  members: (device = false) => request<Member[]>('/api/v1/members', { bearer: device }),
  createMember: (b: CreateMemberReq) =>
    request<MemberDto>('/api/v1/members', { method: 'POST', body: b, ...U }),
  patchMember: (id: string, version: number, b: PatchMemberReq) =>
    request<MemberDto>(`/api/v1/members/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  /** Raw body with the file's own Content-Type (png, jpeg or webp, at most 512 KB). */
  putAvatar: (id: string, file: Blob) =>
    request<void>(`/api/v1/members/${id}/avatar`, { method: 'PUT', raw: file, ...U }),
  deleteAvatar: (id: string) =>
    request<void>(`/api/v1/members/${id}/avatar`, { method: 'DELETE', ...U }),
  devices: () => request<DeviceDto[]>('/api/v1/devices', U),
  pairDevice: (name: string) =>
    request<PairDeviceResp>('/api/v1/devices', { method: 'POST', body: { name }, ...U }),
  revokeDevice: (id: string) => request<void>(`/api/v1/devices/${id}`, { method: 'DELETE', ...U }),
  elevate: (member_id: string, pin: string) =>
    http.post<S['ElevateResp']>('/api/v1/device/elevate', { member_id, pin }),

  household: () => request<HouseholdViewExt>('/api/v1/household', U),
  patchHousehold: (version: number, b: HouseholdPatch) =>
    request<HouseholdViewExt>('/api/v1/household', {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),

  createAccount: (id: string, username: string, password: string) =>
    request<void>(`/api/v1/members/${id}/account`, {
      method: 'POST',
      body: { username, password },
      ...U,
    }),
  /** An adult resetting a child's password (no current password). */
  resetPassword: (id: string, password: string) =>
    request<void>(`/api/v1/members/${id}/account/password`, {
      method: 'POST',
      body: { password },
      ...U,
    }),
  /** Your own password (C-03): needs the current one; other sessions are signed out. */
  changeOwnPassword: (id: string, b: OwnPasswordChange) =>
    request<void>(`/api/v1/members/${id}/account/password`, { method: 'POST', body: b, ...U }),
  setPin: (id: string, pin: string) =>
    request<void>(`/api/v1/members/${id}/pin`, { method: 'PUT', body: { pin }, ...U }),

  setup: () => request<SetupStatus>('/api/v1/setup', U),
  setupHousehold: (b: NewSetupHousehold) =>
    request<{ household_id: string }>('/api/v1/setup/household', { method: 'POST', body: b, ...U }),
  setupOwner: (b: NewSetupOwner) =>
    request<MeResponse>('/api/v1/setup/owner', { method: 'POST', body: b, ...U }),
  setupComplete: () => request<void>('/api/v1/setup/complete', { method: 'POST', ...U }),
  masterKey: (password: string) =>
    request<S['MasterKeyView']>('/api/v1/system/master-key', {
      method: 'POST',
      body: { password },
      ...U,
    }),
  confirmMasterKey: () =>
    request<void>('/api/v1/system/master-key/confirm', { method: 'POST', ...U }),
  about: () => request<AboutView>('/api/v1/system/about', U),
  systemSettings: () => request<SystemSettingsView>('/api/v1/system/settings', U),
  patchSystemSettings: (values: SystemSettingsPatch['values']) =>
    request<SystemSettingsView>('/api/v1/system/settings', {
      method: 'PATCH',
      body: { values },
      ...U,
    }),
  exportConfig: () => request<string>('/api/v1/config/export', { asText: true, ...U }),
  importConfig: (toml: string, dryRun: boolean) =>
    request<ImportReport>(`/api/v1/config/import?dry_run=${dryRun}`, {
      method: 'POST',
      text: toml,
      ...U,
    }),
  /** Liveness of the server after a settings change that restarts parts of it. */
  readyz: () => request<unknown>('/readyz', U),

  /** `from`/`to` are RFC 3339 instants; `to` is exclusive. */
  events: (from: string, to: string, member?: string) => {
    const q = new URLSearchParams({ from, to })
    if (member) q.set('member', member)
    return request<EventView[]>(`/api/v1/events?${q}`, U)
  },
  weather: () => request<WeatherView>('/api/v1/weather', U),
  advisories: () => request<AdvisoryView[]>('/api/v1/advisories', U),
  /** Kiosk snapshot (device token). */
  dashboard: (days: number, signal?: AbortSignal) =>
    request<DashboardView>(`/api/v1/dashboard?days=${days}`, { signal }),
  device: (id: string) => request<DeviceView>(`/api/v1/devices/${id}`, U),
  patchDevice: (id: string, version: number, patch: DevicePatch) =>
    request<DeviceView>(`/api/v1/devices/${id}`, {
      method: 'PATCH',
      body: patch,
      headers: { 'If-Match': String(version) },
      ...U,
    }),

  sources: () => request<SourceViewExt[]>('/api/v1/calendar-sources', U),
  createSource: (b: NewSource) =>
    request<SourceCreated>('/api/v1/calendar-sources', { method: 'POST', body: b, ...U }),
  testSource: (b: NewSource) =>
    request<SourceTestResult>('/api/v1/calendar-sources/test', { method: 'POST', body: b, ...U }),
  patchSource: (id: string, version: number, b: SourcePatch) =>
    request<SourceView>(`/api/v1/calendar-sources/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  deleteSource: (id: string) =>
    request<void>(`/api/v1/calendar-sources/${id}`, { method: 'DELETE', ...U }),
  syncSource: (id: string) =>
    request<void>(`/api/v1/calendar-sources/${id}/sync`, { method: 'POST', ...U }),
  calendars: () => request<CalendarViewExt[]>('/api/v1/calendars', U),
  patchCalendar: (id: string, version: number, b: CalendarPatch) =>
    request<CalendarView>(`/api/v1/calendars/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  oauthAccounts: () => request<OAuthAccountView[]>('/api/v1/oauth/accounts', U),
  /** Full-page navigation target: the server answers with a 302 to Google. */
  googleStartUrl: (
    returnTo: string,
    purpose?: 'calendar' | 'gmail_read' | 'gmail_modify' | 'calendar_write',
  ) => {
    const p = new URLSearchParams({ return_to: returnTo })
    if (purpose) p.set('purpose', purpose)
    return `/api/v1/oauth/google/start?${p}`
  },

  rules: () => request<WeatherRuleView[]>('/api/v1/weather-rules', U),
  createRule: (b: NewWeatherRule) =>
    request<WeatherRuleView>('/api/v1/weather-rules', { method: 'POST', body: b, ...U }),
  patchRule: (id: string, version: number, b: WeatherRulePatch) =>
    request<WeatherRuleView>(`/api/v1/weather-rules/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  deleteRule: (id: string) =>
    request<void>(`/api/v1/weather-rules/${id}`, { method: 'DELETE', ...U }),
  validateRule: (condition: string, message?: string) =>
    request<ValidateRuleResponse>('/api/v1/weather-rules/validate', {
      method: 'POST',
      body: { condition, message: message ?? null },
      ...U,
    }),

  status: () => request<StatusView>('/api/v1/status', U),

  tasks: (q: TaskQuery = {}) => {
    const p = new URLSearchParams()
    for (const [k, v] of Object.entries(q)) if (v) p.set(k, v)
    const qs = p.toString()
    return request<TaskView[]>(`/api/v1/tasks${qs ? `?${qs}` : ''}`, U)
  },
  task: (id: string) => request<TaskView>(`/api/v1/tasks/${id}`, U),
  createTask: (b: NewTask) => request<TaskView>('/api/v1/tasks', { method: 'POST', body: b, ...U }),
  patchTask: (id: string, version: number, b: TaskPatch) =>
    request<TaskView>(`/api/v1/tasks/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  deleteTask: (id: string) => request<void>(`/api/v1/tasks/${id}`, { method: 'DELETE', ...U }),
  completeTask: (id: string, member?: string) =>
    request<TaskView>(`/api/v1/tasks/${id}/complete`, {
      method: 'POST',
      body: { member: member ?? null },
      ...U,
    }),
  reopenTask: (id: string, member?: string) =>
    request<TaskView>(`/api/v1/tasks/${id}/reopen`, {
      method: 'POST',
      body: { member: member ?? null },
      ...U,
    }),
  skipTask: (id: string) => request<TaskView>(`/api/v1/tasks/${id}/skip`, { method: 'POST', ...U }),
  acceptTask: (id: string) =>
    request<TaskView>(`/api/v1/tasks/${id}/accept`, { method: 'POST', ...U }),
  claimTask: (id: string) =>
    request<TaskView>(`/api/v1/tasks/${id}/claim`, { method: 'POST', ...U }),
  releaseTask: (id: string) =>
    request<TaskView>(`/api/v1/tasks/${id}/release`, { method: 'POST', ...U }),
  tickTaskStep: (id: string, stepId: string, done: boolean, member?: string) =>
    request<TaskView>(`/api/v1/tasks/${id}/steps/${stepId}`, {
      method: 'PATCH',
      body: { done, member: member ?? null },
      ...U,
    }),

  taskLists: () => request<TaskListView[]>('/api/v1/task-lists', U),
  createTaskList: (b: TaskListInput) =>
    request<TaskListView>('/api/v1/task-lists', { method: 'POST', body: b, ...U }),
  patchTaskList: (id: string, version: number, b: TaskListPatch) =>
    request<TaskListView>(`/api/v1/task-lists/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  deleteTaskList: (id: string) =>
    request<void>(`/api/v1/task-lists/${id}`, { method: 'DELETE', ...U }),

  taskDefinitions: (list?: string) =>
    request<TaskDefinitionView[]>(`/api/v1/task-definitions${list ? `?list=${list}` : ''}`, U),
  createTaskDefinition: (b: TaskDefinitionInput) =>
    request<TaskDefinitionView>('/api/v1/task-definitions', { method: 'POST', body: b, ...U }),
  patchTaskDefinition: (id: string, version: number, b: TaskDefinitionPatch) =>
    request<TaskDefinitionView>(`/api/v1/task-definitions/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  deleteTaskDefinition: (id: string) =>
    request<void>(`/api/v1/task-definitions/${id}`, { method: 'DELETE', ...U }),
  validateDefinition: (b: {
    rrule: string | null
    season_condition: string | null
    start_date?: string | null
  }) =>
    request<ValidateDefinitionResponse>('/api/v1/task-definitions/validate', {
      method: 'POST',
      body: b,
      ...U,
    }),

  routines: () => request<RoutineView[]>('/api/v1/routines', U),
  createRoutine: (b: RoutineInput) =>
    request<RoutineView>('/api/v1/routines', { method: 'POST', body: b, ...U }),
  patchRoutine: (id: string, version: number, b: RoutinePatch) =>
    request<RoutineView>(`/api/v1/routines/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  deleteRoutine: (id: string) =>
    request<void>(`/api/v1/routines/${id}`, { method: 'DELETE', ...U }),
  routinesToday: (member?: string) =>
    request<RoutineRunView[]>(`/api/v1/routines/today${member ? `?member=${member}` : ''}`, U),
  tickRoutineStep: (runId: string, stepId: string, done: boolean, member?: string) =>
    request<RoutineRunView>(`/api/v1/routine-runs/${runId}/steps/${stepId}`, {
      method: 'PATCH',
      body: { done, member: member ?? null },
      ...U,
    }),

  away: (member: string) => request<MemberAwayView[]>(`/api/v1/members/${member}/away`, U),
  addAway: (member: string, b: NewAway) =>
    request<MemberAwayView>(`/api/v1/members/${member}/away`, { method: 'POST', body: b, ...U }),
  deleteAway: (id: string) => request<void>(`/api/v1/away/${id}`, { method: 'DELETE', ...U }),

  notifyPrefs: (member: string) =>
    request<NotifyPrefView>(`/api/v1/members/${member}/notify-prefs`, U),
  putNotifyPrefs: (member: string, version: number, b: NotifyPrefPatch) =>
    request<NotifyPrefView>(`/api/v1/members/${member}/notify-prefs`, {
      method: 'PUT',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  testNotify: (member: string) =>
    request<void>(`/api/v1/members/${member}/notify-prefs/test`, { method: 'POST', ...U }),

  messages: (q: MessageQuery = {}) => {
    const p = new URLSearchParams()
    for (const [k, v] of Object.entries(q))
      if (v !== undefined && v !== '' && v !== false) p.set(k, String(v))
    const qs = p.toString()
    return request<MessagePage>(`/api/v1/messages${qs ? `?${qs}` : ''}`, U)
  },
  message: (id: string, member?: string) =>
    request<MessageDetail>(`/api/v1/messages/${id}${member ? `?member=${member}` : ''}`, U),
  mailSources: () => request<MailSourceView[]>('/api/v1/mail-sources', U),
  categories: () => request<CategoryView[]>('/api/v1/categories', U),
  messageAction: (id: string, b: MailActionInput) =>
    request<MailActionView>(`/api/v1/messages/${id}/actions`, { method: 'POST', body: b, ...U }),
  undoMailAction: (id: string) =>
    request<MailActionView>(`/api/v1/mail-actions/${id}/undo`, { method: 'POST', ...U }),
  labelMessage: (id: string, b: LabelInput) =>
    request<MessageDetail>(`/api/v1/messages/${id}/labels`, { method: 'POST', body: b, ...U }),
  setEvalSample: (id: string, inSample: boolean) =>
    request<void>(`/api/v1/messages/${id}/eval-sample`, {
      method: 'POST',
      body: { in_sample: inSample },
      ...U,
    }),
  makeTask: (id: string, b: MakeTaskInput = {}) =>
    request<void>(`/api/v1/messages/${id}/task`, { method: 'POST', body: b, ...U }),
  /** Resolves to `undefined` when the queue is empty (204). */
  labellingNext: (after?: string) =>
    request<LabelQueueItem | undefined>(
      `/api/v1/labelling/next${after ? `?after=${after}` : ''}`,
      U,
    ),

  inbox: (kind?: InboxKind) => request<InboxView>(`/api/v1/inbox${kind ? `?kind=${kind}` : ''}`, U),
  approveInbox: (
    kind: InboxKind,
    id: string,
    b: EventApproval | BillPatch | TaskApproval | ContactApproval = {},
  ) => request<void>(`/api/v1/inbox/${kind}/${id}/approve`, { method: 'POST', body: b, ...U }),
  rejectInbox: (kind: InboxKind, id: string) =>
    request<void>(`/api/v1/inbox/${kind}/${id}/reject`, { method: 'POST', ...U }),

  bills: (status?: string) =>
    request<BillView[]>(`/api/v1/bills${status ? `?status=${status}` : ''}`, U),
  patchBill: (id: string, version: number, b: BillPatch) =>
    request<BillView>(`/api/v1/bills/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
}
