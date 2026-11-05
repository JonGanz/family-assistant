<script setup lang="ts">
// List-detail (Frontend.md §3c). Compact and medium: /settings is the grouped list and a child page
// fills the screen (its SettingsPage supplies the back arrow). Expanded: list left, page right.
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useSessionStore } from '../stores/session'
import { UiList, UiListItem, UiIcon, UiSection, UiTopAppBar, useBreakpoint } from '../ui'
import type { IconName } from '../ui'
import { visibleGroups, type SettingsPage } from './settingsNav'

const session = useSessionStore()
const route = useRoute()
const bp = useBreakpoint()
const groups = computed(() => visibleGroups(session.isAdult))
const expanded = computed(() => bp.value === 'expanded')
const isIndex = computed(() => route.path === '/settings' || route.path === '/settings/')

// Until settingsNav.ts carries an icon per page (T-13), every row gets the settings glyph.
const iconFor = (p: SettingsPage): IconName => (p as { icon?: IconName }).icon ?? 'settings'
const showList = computed(() => expanded.value || isIndex.value)
const showPage = computed(() => expanded.value || !isIndex.value)
const isActive = (to: string) => route.path === to || route.path.startsWith(`${to}/`)
</script>

<template>
  <div class="settings-layout" :class="{ 'is-split': expanded }">
    <div v-if="showList" class="settings-list">
      <UiTopAppBar v-if="!expanded" title="Settings" />
      <nav aria-label="Settings" class="settings-nav">
        <UiSection v-for="g in groups" :key="g.title" :heading="g.title">
          <UiList>
            <UiListItem
              v-for="p in g.pages"
              :key="p.to"
              :headline="p.label"
              :supporting="expanded ? undefined : p.blurb"
              :to="p.to"
              :class="{ 'is-current': expanded && isActive(p.to) }"
            >
              <template #leading><UiIcon :name="iconFor(p)" /></template>
            </UiListItem>
          </UiList>
        </UiSection>
      </nav>
    </div>
    <div v-if="showPage" class="settings-page"><RouterView /></div>
  </div>
</template>

<style scoped>
.settings-nav {
  display: grid;
  gap: var(--ui-space-4);
  padding: var(--ui-space-2) var(--ui-space-4) var(--ui-space-4);
}
.settings-layout.is-split {
  display: grid;
  grid-template-columns: 320px minmax(0, 1fr);
  align-items: start;
}
.is-split .settings-list {
  position: sticky;
  top: 0;
  max-height: 100dvh;
  overflow-y: auto;
  border-right: 1px solid var(--ui-outline-variant);
}
.is-split .settings-nav {
  padding-top: var(--ui-space-4);
}
/* Pages not yet on SettingsPage keep the old padding until they are migrated. */
.settings-page > :deep(:not(.ui-page)) {
  box-sizing: border-box;
  max-width: 64rem;
  margin: 0 auto;
  padding: 1rem;
}
.is-current :deep(.ui-li__row) {
  background: var(--ui-primary-container);
  border-radius: var(--ui-radius-full);
}
</style>
