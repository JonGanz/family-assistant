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

  household: () => request<HouseholdView>('/api/v1/household', U),
  patchHousehold: (version: number, b: HouseholdPatch) =>
    request<HouseholdView>('/api/v1/household', {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),

  /** `from`/`to` are RFC 3339 instants; `to` is exclusive. */
  events: (from: string, to: string, member?: string) => {
    const q = new URLSearchParams({ from, to })
    if (member) q.set('member', member)
    return request<EventView[]>(`/api/v1/events?${q}`, U)
  },
  weather: () => request<WeatherView>('/api/v1/weather', U),
  advisories: () => request<AdvisoryView[]>('/api/v1/advisories', U),
  /** `device` sends the kiosk's bearer token instead of the session cookie. */
  dismissAdvisory: (id: string, device = false) =>
    request<void>(`/api/v1/advisories/${id}/dismiss`, { method: 'POST', bearer: device }),
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

  sources: () => request<SourceView[]>('/api/v1/calendar-sources', U),
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
  calendars: () => request<CalendarView[]>('/api/v1/calendars', U),
  patchCalendar: (id: string, version: number, b: CalendarPatch) =>
    request<CalendarView>(`/api/v1/calendars/${id}`, {
      method: 'PATCH',
      body: b,
      headers: ifMatch(version),
      ...U,
    }),
  oauthAccounts: () => request<OAuthAccountView[]>('/api/v1/oauth/accounts', U),
  /** Full-page navigation target: the server answers with a 302 to Google. */
  googleStartUrl: (returnTo: string) =>
    `/api/v1/oauth/google/start?${new URLSearchParams({ return_to: returnTo })}`,

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
}
