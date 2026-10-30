import type { components } from '../gen/api'
import type { PanelConfig } from './layout'

type Schemas = components['schemas']
export type TaskView = Schemas['TaskView']
export type RoutineRunView = Schemas['RoutineRunView']
export type TaskListView = Schemas['TaskListView']

/** Client-side additions the store merges into the server snapshot. */
export interface DashboardExtras {
  /** Task lists (for the add form); read with the device token. */
  task_lists?: TaskListView[]
  /** The household's Now list size; falls back to 7. */
  now_list_size?: number
  /** False once the server refused a device completion (override off): tasks become read-only. */
  completion_enabled?: boolean
  /** Bumped to reset uncontrolled checkboxes after a cancelled picker. */
  epoch?: number
}
export type DashboardData = Schemas['DashboardView'] & DashboardExtras
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

/** What a panel asks the canvas to show in a modal. */
export type DetailRequest =
  | { kind: 'weather' }
  | { kind: 'event'; id: string }
  /** Everything one Week-view cell holds (the "+N" slot): event and task ids, and its colour. */
  | { kind: 'day'; title: string; color: string; events: string[]; tasks: string[] }
