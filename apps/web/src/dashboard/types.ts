import type { components } from '../gen/api'
import type { PanelConfig } from './layout'

type Schemas = components['schemas']
export type DashboardData = Schemas['DashboardView']
export type DashboardSettings = Schemas['DashboardSettings']
export type EventView = Schemas['EventView']
export type AdvisoryView = Schemas['AdvisoryView']
export type MemberPublic = Schemas['MemberPublicDto']
export type WeatherView = Schemas['WeatherView']
export type DeviceView = Schemas['DeviceView']
export type DevicePatch = Schemas['DevicePatch']
export type HouseholdView = Schemas['HouseholdView']

/** What every panel receives. Panels are pure: they render props and emit events. */
export interface PanelProps {
  data: DashboardData
  /** Settings being rendered (the editor passes edited ones, not data.settings). */
  settings: DashboardSettings
  panel: PanelConfig
  now: Date
}
