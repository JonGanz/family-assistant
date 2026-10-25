import { request } from '../api/http'
import type { components } from '../gen/api'

type S = components['schemas']
export type TaskView = S['TaskView']
export type TaskListView = S['TaskListView']
export type HouseholdView = S['HouseholdView']
export type NewTask = S['NewTask']

const post = <T>(path: string, body: unknown) => request<T>(path, { method: 'POST', body })

/** Calls the dashboard makes with the device token (the main api client belongs to the app shell). */
export const taskApi = {
  complete: (id: string, member: string) =>
    post<TaskView>(`/api/v1/tasks/${id}/complete`, { member }),
  reopen: (id: string, member: string) => post<TaskView>(`/api/v1/tasks/${id}/reopen`, { member }),
  create: (b: NewTask) => post<TaskView>('/api/v1/tasks', b),
  tickStep: (run: string, step: string, done: boolean, member: string) =>
    request<S['RoutineRunView']>(`/api/v1/routine-runs/${run}/steps/${step}`, {
      method: 'PATCH',
      body: { done, member },
    }),
  /** `bearer: false` reads with the user session (the layout editor). */
  lists: (bearer = true) => request<TaskListView[]>('/api/v1/task-lists', { bearer }),
  household: () => request<HouseholdView>('/api/v1/household'),
}
