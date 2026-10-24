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

/** Adults get every field; children and devices only the public ones. */
export type Member = MemberPublicDto & Partial<Omit<MemberDto, keyof MemberPublicDto>>

type Body<K extends keyof operations> = operations[K] extends {
  requestBody: { content: { 'application/json': infer B } }
}
  ? B
  : never

const U = { bearer: false } as const

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
      headers: { 'If-Match': String(version) },
      ...U,
    }),
  devices: () => request<DeviceDto[]>('/api/v1/devices', U),
  pairDevice: (name: string) =>
    request<PairDeviceResp>('/api/v1/devices', { method: 'POST', body: { name }, ...U }),
  revokeDevice: (id: string) => request<void>(`/api/v1/devices/${id}`, { method: 'DELETE', ...U }),
  elevate: (member_id: string, pin: string) =>
    http.post<S['ElevateResp']>('/api/v1/device/elevate', { member_id, pin }),
}
