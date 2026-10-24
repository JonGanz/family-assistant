import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
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
    path: '/members',
    name: 'members',
    component: () => import('../views/MembersView.vue'),
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

export const router = createRouter({ history: createWebHistory(), routes })

router.beforeEach(async (to) => {
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
})
