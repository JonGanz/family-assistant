<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useNavigation } from '../app/navigation'
import { useInboxStore } from '../stores/inbox'
import { useSessionStore } from '../stores/session'
import {
  UiBottomSheet,
  UiButton,
  UiIconButton,
  UiList,
  UiListItem,
  UiNavBar,
  UiNavDrawer,
  UiNavRail,
  UiSnackbarHost,
  isActivePath,
  useBreakpoint,
  type NavDestination,
} from '../ui'

const session = useSessionStore()
const router = useRouter()
const route = useRoute()
const inbox = useInboxStore()
const bp = useBreakpoint()
const nav = useNavigation()
const moreOpen = ref(false)

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
  () => (moreOpen.value = false),
)

/** The bar: the primary destinations plus a More button that opens the sheet. */
const barItems = computed<NavDestination[]>(() => [
  ...nav.value.primary,
  {
    id: 'more',
    label: 'More',
    icon: 'more',
    to: route.path,
    action: () => (moreOpen.value = true),
    current: moreOpen.value || nav.value.more.some((d) => isActivePath(route.path, d.to)),
  },
])
/** The drawer groups the destinations the way the bar and More do. */
const drawerItems = computed<NavDestination[]>(() =>
  nav.value.all.map((d) => ({ ...d, group: d.primary ? '' : 'More' })),
)

async function logout() {
  moreOpen.value = false
  await session.logout()
  await router.replace({ name: 'login' })
}

const withNav = computed(() => session.isUser)
</script>

<template>
  <div class="app-layout" :class="[`is-${bp}`, { 'has-nav': withNav }]">
    <a class="skip-link" href="#main">Skip to content</a>
    <aside v-if="withNav && bp === 'medium'" class="app-side">
      <UiNavRail :destinations="nav.all" />
      <UiIconButton icon="logout" label="Log out" @click="logout" />
    </aside>
    <aside v-else-if="withNav && bp === 'expanded'" class="app-side">
      <UiNavDrawer :destinations="drawerItems" />
      <UiButton variant="text" icon="logout" class="app-logout" @click="logout">Log out</UiButton>
    </aside>
    <div id="main" class="app-main" tabindex="-1"><slot /></div>
    <footer v-if="withNav && bp === 'compact'" class="app-bar">
      <UiNavBar :destinations="barItems" />
    </footer>
    <UiBottomSheet v-model="moreOpen" title="More">
      <UiList>
        <UiListItem
          v-for="d in nav.more"
          :key="d.id"
          :headline="d.label"
          :to="d.to"
          @click="moreOpen = false"
        />
        <UiListItem headline="Log out" @click="logout" />
      </UiList>
    </UiBottomSheet>
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
  --ui-bottom-offset: 0px;
}
.app-layout.is-compact.has-nav {
  /* The bar is 80 px plus the safe area; snackbars and the Save bar sit above it. */
  --ui-bottom-offset: calc(80px + env(safe-area-inset-bottom));
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
/* Views not yet on UiPage keep the old padding and width until they are migrated. */
.app-main > :deep(:not(.ui-page, .settings-layout)) {
  box-sizing: border-box;
  max-width: 64rem;
  margin: 0 auto;
  padding: 1rem;
}
.app-bar {
  position: sticky;
  bottom: 0;
  z-index: 20;
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
.is-expanded .app-side {
  align-items: stretch;
  padding-right: var(--ui-space-3);
}
</style>
