<script setup lang="ts">
// SettingsLayout draws the grouped list. This route only fills the detail pane when the list sits
// beside it (expanded); on phones and tablets the list is the whole screen and this is not shown.
import { UiEmptyState, UiPage, UiTopAppBar } from '../ui'
</script>

<template>
  <UiPage>
    <template #top><UiTopAppBar title="Settings" /></template>
    <UiEmptyState icon="settings" title="Choose a setting" text="Pick a page from the list." />
  </UiPage>
</template>
