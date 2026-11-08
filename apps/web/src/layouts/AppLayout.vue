<script setup lang="ts">
import { computed, onBeforeUnmount, provide, ref, watch } from 'vue'
import I_menu from '~icons/material-symbols/menu-rounded'
import { useRoute, useRouter } from 'vue-router'
import { useNavigation } from '../app/navigation'
import { getNavCollapsed, setNavCollapsed } from '../app/navPrefs'
import { applyUpdate, watchForUpdates } from '../lib/appUpdate'
import { useInboxStore } from '../stores/inbox'
import { useSessionStore } from '../stores/session'
import {
  UiButton,
  UiIconButton,
  UiNavDrawer,
  UiNavRail,
  UiSnackbarHost,
  hasUnsavedForms,
  useBreakpoint,
  useSnackbar,
} from '../ui'
import { navControlKey } from '../ui/nav'

const session = useSessionStore()
const router = useRouter()
const route = useRoute()
const inbox = useInboxStore()
const bp = useBreakpoint()
const nav = useNavigation()
const navOpen = ref(false)
// Read before the first render so the content width does not jump on load.
const collapsed = ref(getNavCollapsed())

provide(navControlKey, { openNav: () => (navOpen.value = true) })

// A deploy (D-123): say so once, then reload by itself on the next foreground return or navigation,
// never over a form with unsaved changes.
const snackbar = useSnackbar()
// The notice waits for a free snackbar, so it never holds back an Undo the user is waiting for.
function showUpdateNotice() {
  const busy = () => !!snackbar.state.current || snackbar.state.queue.length > 0
  const show = () =>
    snackbar.show({
      text: 'A new version is available',
      action: { label: 'Reload', run: () => applyUpdate() },
    })
  if (!busy()) return show()
  const stop = watch(busy, (b) => {
    if (!b) {
      stop()
      show()
    }
  })
}
const updates = watchForUpdates({
  canReload: () => !hasUnsavedForms(),
  onPending: showUpdateNotice,
})
const stopNavHook = router.afterEach(() => updates.tryNow())
onBeforeUnmount(() => {
  updates.stop()
  stopNavHook()
})

// Adults only: the inbox badge loads once and then follows SSE `inbox.updated` (stores/live.ts).
watch(
  () => session.isAdult,
  (adult) => {
    if (adult) void inbox.load()
  },
  { immediate: true },
)
watch(
  () => route.path,
  () => (navOpen.value = false),
)
watch(bp, () => (navOpen.value = false))

/** The drawer groups the destinations the way the old bar and More did; Food shows its tabs as its own group (M5). */
const drawerItems = computed(() =>
  nav.value.drawer.map((d) => ({ ...d, group: d.group ?? (d.primary ? '' : 'More') })),
)

function setCollapsed(v: boolean) {
  collapsed.value = v
  setNavCollapsed(v)
}

async function logout() {
  navOpen.value = false
  await session.logout()
  await router.replace({ name: 'login' })
}

const withNav = computed(() => session.isUser)
const showRail = computed(
  () => withNav.value && (bp.value === 'medium' || (bp.value === 'expanded' && collapsed.value)),
)
const showDrawer = computed(() => withNav.value && bp.value === 'expanded' && !collapsed.value)
</script>

<template>
  <div class="app-layout" :class="[`is-${bp}`, { 'has-nav': withNav }]">
    <a class="skip-link" href="#main">Skip to content</a>
    <aside v-if="showRail" class="app-side">
      <button
        v-if="bp === 'expanded'"
        type="button"
        class="app-expand"
        aria-label="Expand navigation"
        title="Expand navigation"
        aria-expanded="false"
        @click="setCollapsed(false)"
      >
        <I_menu aria-hidden="true" />
      </button>
      <UiNavRail :destinations="nav.all" />
      <UiIconButton icon="logout" label="Log out" @click="logout" />
    </aside>
    <aside v-else-if="showDrawer" class="app-side">
      <UiNavDrawer :destinations="drawerItems" collapsible @collapse="setCollapsed(true)">
        <template #footer>
          <UiButton variant="text" icon="logout" class="app-logout" @click="logout">
            Log out
          </UiButton>
        </template>
      </UiNavDrawer>
    </aside>
    <div id="main" class="app-main" tabindex="-1"><slot /></div>
    <UiNavDrawer
      v-if="withNav && bp === 'compact'"
      modal
      :open="navOpen"
      :destinations="drawerItems"
      @close="navOpen = false"
    >
      <template #footer>
        <UiButton variant="text" icon="logout" class="app-logout" @click="logout">
          Log out
        </UiButton>
      </template>
    </UiNavDrawer>
    <UiSnackbarHost />
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  flex-direction: column;
  min-height: 100dvh;
  background: var(--ui-surface);
  color: var(--ui-on-surface);
  /* No bottom bar on any window class: snackbars and the Save bar sit at the edge. */
  --ui-bottom-offset: 0px;
}
.app-layout.is-medium,
.app-layout.is-expanded {
  flex-direction: row;
}
.app-main {
  flex: 1;
  min-width: 0;
  outline: none;
}
.app-side {
  position: sticky;
  top: 0;
  z-index: 20;
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: none;
  height: 100dvh;
  padding-left: env(safe-area-inset-left);
  overflow-y: auto;
  background: var(--ui-surface-container-low);
}
.app-side > :first-child {
  flex: none;
}
.app-side > :last-child {
  margin: auto 0 var(--ui-space-4);
}
.app-side > .ui-drawer-wrap {
  flex: 1;
  margin: 0;
}
.is-expanded .app-side {
  align-items: stretch;
  padding-right: var(--ui-space-3);
}
.is-expanded .app-side:has(.ui-rail) {
  align-items: center;
  padding-right: 0;
}
.app-expand {
  display: flex;
  align-items: center;
  justify-content: center;
  width: var(--ui-touch);
  height: var(--ui-touch);
  margin-top: var(--ui-space-2);
  border: 0;
  border-radius: var(--ui-radius-full);
  background: none;
  color: var(--ui-on-surface-variant);
  font-size: 1.5rem;
  cursor: pointer;
}
.app-expand:hover {
  background: var(--ui-state-hover);
}
.app-expand:focus-visible {
  outline: var(--ui-focus-ring);
}
.app-logout {
  width: 100%;
}
</style>
