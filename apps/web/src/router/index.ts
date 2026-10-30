import {
  createRouter,
  createWebHistory,
  type RouteLocationNormalized,
  type RouteLocationRaw,
  type RouteRecordRaw,
} from 'vue-router'
import { onSetupRequired } from '../api/http'
import { captureDeviceToken } from '../kiosk/deviceToken'
import { useSessionStore } from '../stores/session'
import { useSetupStore } from '../stores/setup'

declare module 'vue-router' {
  interface RouteMeta {
    layout?: 'app' | 'kiosk' | 'setup'
    auth?: 'user' | 'adult' | 'device'
  }
}

const adult = { auth: 'adult' } as const
const user = { auth: 'user' } as const

export const routes: RouteRecordRaw[] = [
  {
    path: '/setup',
    name: 'setup',
    component: () => import('../views/SetupView.vue'),
    meta: { layout: 'setup' },
  },
  { path: '/login', name: 'login', component: () => import('../views/LoginView.vue') },
  { path: '/', name: 'home', component: () => import('../views/HomeView.vue'), meta: user },
  {
    path: '/calendar',
    name: 'calendar',
    component: () => import('../views/CalendarView.vue'),
    meta: user,
  },
  { path: '/tasks', name: 'tasks', component: () => import('../views/TasksView.vue'), meta: user },
  {
    path: '/tasks/:id',
    name: 'task-detail',
    component: () => import('../views/TaskDetailView.vue'),
    meta: user,
  },
  {
    path: '/routines',
    name: 'routines',
    component: () => import('../views/RoutinesView.vue'),
    meta: user,
  },
  { path: '/mail', name: 'mail', component: () => import('../views/MailView.vue'), meta: adult },
  {
    path: '/mail/label',
    name: 'mail-label',
    component: () => import('../views/LabelView.vue'),
    meta: adult,
  },
  {
    path: '/mail/:id',
    name: 'mail-detail',
    component: () => import('../views/MessageView.vue'),
    meta: adult,
  },
  { path: '/inbox', name: 'inbox', component: () => import('../views/InboxView.vue'), meta: adult },
  { path: '/bills', name: 'bills', component: () => import('../views/BillsView.vue'), meta: adult },
  {
    path: '/oauth/done',
    name: 'oauth-done',
    component: () => import('../views/OAuthDoneView.vue'),
    meta: user,
  },
  {
    path: '/settings',
    component: () => import('../views/SettingsLayout.vue'),
    meta: user,
    children: [
      {
        path: '',
        name: 'settings',
        component: () => import('../views/SettingsIndexView.vue'),
        meta: user,
      },
      {
        path: 'account',
        name: 'settings-account',
        component: () => import('../views/SettingsAccountView.vue'),
        meta: user,
      },
      {
        path: 'appearance',
        name: 'settings-appearance',
        component: () => import('../views/SettingsAppearanceView.vue'),
        meta: user,
      },
      {
        path: 'notifications',
        name: 'settings-notifications',
        component: () => import('../views/SettingsNotificationsView.vue'),
        meta: user,
      },
      {
        path: 'household',
        name: 'settings-household',
        component: () => import('../views/SettingsHouseholdView.vue'),
        meta: adult,
      },
      {
        path: 'members',
        name: 'settings-members',
        component: () => import('../views/SettingsMembersView.vue'),
        meta: adult,
      },
      {
        path: 'tasks',
        name: 'settings-tasks',
        component: () => import('../views/SettingsTasksView.vue'),
        meta: adult,
      },
      {
        path: 'calendars',
        name: 'settings-calendars',
        component: () => import('../views/SettingsCalendarsView.vue'),
        meta: adult,
      },
      {
        path: 'weather',
        name: 'settings-weather',
        component: () => import('../views/SettingsWeatherView.vue'),
        meta: adult,
      },
      {
        path: 'mail',
        name: 'settings-mail',
        component: () => import('../views/SettingsMailView.vue'),
        meta: adult,
      },
      {
        path: 'orgs',
        name: 'settings-orgs',
        component: () => import('../views/SettingsOrgsView.vue'),
        meta: adult,
      },
      {
        path: 'system',
        name: 'settings-system',
        component: () => import('../views/SettingsSystemView.vue'),
        meta: adult,
      },
      {
        path: 'devices',
        name: 'settings-devices',
        component: () => import('../views/DevicesView.vue'),
        meta: adult,
      },
      {
        path: 'about',
        name: 'settings-about',
        component: () => import('../views/SettingsAboutView.vue'),
        meta: adult,
      },
      {
        path: 'status',
        name: 'settings-status',
        component: () => import('../views/SettingsStatusView.vue'),
        meta: adult,
      },
    ],
  },
  {
    path: '/dashboard',
    name: 'dashboard',
    component: () => import('../views/DashboardView.vue'),
    meta: { layout: 'kiosk', auth: 'device' },
  },
]

/**
 * Route guard: returns `true` to continue or a redirect location.
 *
 * Setup comes first: while no household or no owner exists every route goes to `/setup`, and so
 * does an adult until the wizard is finished. A child is never sent there.
 */
export async function authGuard(to: RouteLocationNormalized): Promise<true | RouteLocationRaw> {
  const auth = to.meta.auth
  if (auth === 'device') {
    // The view shows a "pair this screen" notice when no token is present.
    captureDeviceToken()
    return true
  }
  const setup = useSetupStore()
  const status = await setup.load()
  const toSetup = to.name === 'setup'
  if (status && (status.needed || status.owner_needed)) return toSetup ? true : { name: 'setup' }

  const session = useSessionStore()
  if (toSetup) {
    if (!status) return true
    await session.load()
    return session.isAdult && !status.completed ? true : { name: 'home' }
  }
  if (!auth) return true
  await session.load()
  if (!session.isUser) return { name: 'login', query: { redirect: to.fullPath } }
  if (status && !status.completed && session.isAdult) return { name: 'setup' }
  if (auth === 'adult' && !session.isAdult) return { name: 'home' }
  return true
}

export const router = createRouter({ history: createWebHistory(), routes })

router.beforeEach(authGuard)

// Any `409 setup_required` answer sends the app back to the wizard.
onSetupRequired(() => {
  useSetupStore().invalidate()
  if (router.currentRoute.value.name !== 'setup') void router.replace({ name: 'setup' })
})
