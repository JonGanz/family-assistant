import {
  createRouter,
  createWebHistory,
  type RouteLocationNormalized,
  type RouteLocationRaw,
  type RouteRecordRaw,
} from 'vue-router'
import { captureDeviceToken } from '../kiosk/deviceToken'
import { useSessionStore } from '../stores/session'

declare module 'vue-router' {
  interface RouteMeta {
    layout?: 'app' | 'kiosk'
    auth?: 'user' | 'adult' | 'device'
  }
}

export const routes: RouteRecordRaw[] = [
  { path: '/login', name: 'login', component: () => import('../views/LoginView.vue') },
  {
    path: '/',
    name: 'home',
    component: () => import('../views/HomeView.vue'),
    meta: { auth: 'user' },
  },
  {
    path: '/calendar',
    name: 'calendar',
    component: () => import('../views/CalendarView.vue'),
    meta: { auth: 'user' },
  },
  {
    path: '/tasks',
    name: 'tasks',
    component: () => import('../views/TasksView.vue'),
    meta: { auth: 'user' },
  },
  {
    path: '/tasks/:id',
    name: 'task-detail',
    component: () => import('../views/TaskDetailView.vue'),
    meta: { auth: 'user' },
  },
  {
    path: '/routines',
    name: 'routines',
    component: () => import('../views/RoutinesView.vue'),
    meta: { auth: 'user' },
  },
  {
    path: '/settings/notifications',
    name: 'settings-notifications',
    component: () => import('../views/SettingsNotificationsView.vue'),
    meta: { auth: 'user' },
  },
  {
    path: '/members',
    name: 'members',
    component: () => import('../views/MembersView.vue'),
    meta: { auth: 'user' },
  },
  { path: '/settings', redirect: { name: 'settings-household' } },
  {
    path: '/settings/household',
    name: 'settings-household',
    component: () => import('../views/SettingsHouseholdView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/settings/tasks',
    name: 'settings-tasks',
    component: () => import('../views/SettingsTasksView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/settings/calendars',
    name: 'settings-calendars',
    component: () => import('../views/SettingsCalendarsView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/settings/weather',
    name: 'settings-weather',
    component: () => import('../views/SettingsWeatherView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/settings/status',
    name: 'settings-status',
    component: () => import('../views/SettingsStatusView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/mail',
    name: 'mail',
    component: () => import('../views/MailView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/mail/label',
    name: 'mail-label',
    component: () => import('../views/LabelView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/mail/:id',
    name: 'mail-detail',
    component: () => import('../views/MessageView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/inbox',
    name: 'inbox',
    component: () => import('../views/InboxView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/bills',
    name: 'bills',
    component: () => import('../views/BillsView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/settings/mail',
    name: 'settings-mail',
    component: () => import('../views/SettingsMailView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/settings/orgs',
    name: 'settings-orgs',
    component: () => import('../views/SettingsOrgsView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/oauth/done',
    name: 'oauth-done',
    component: () => import('../views/OAuthDoneView.vue'),
    meta: { auth: 'user' },
  },
  {
    path: '/settings/devices',
    name: 'devices',
    component: () => import('../views/DevicesView.vue'),
    meta: { auth: 'adult' },
  },
  {
    path: '/dashboard',
    name: 'dashboard',
    component: () => import('../views/DashboardView.vue'),
    meta: { layout: 'kiosk', auth: 'device' },
  },
]

/** Route guard: returns `true` to continue or a redirect location. */
export async function authGuard(to: RouteLocationNormalized): Promise<true | RouteLocationRaw> {
  const auth = to.meta.auth
  if (auth === 'device') {
    // The view shows a "pair this screen" notice when no token is present.
    captureDeviceToken()
    return true
  }
  if (!auth) return true
  const session = useSessionStore()
  await session.load()
  if (!session.isUser) return { name: 'login', query: { redirect: to.fullPath } }
  if (auth === 'adult' && !session.isAdult) return { name: 'home' }
  return true
}

export const router = createRouter({ history: createWebHistory(), routes })

router.beforeEach(authGuard)
