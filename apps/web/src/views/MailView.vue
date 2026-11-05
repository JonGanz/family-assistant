<script setup lang="ts">
import { onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import MessageRow from '../components/mail/MessageRow.vue'
import { useHouseholdStore } from '../stores/household'
import { useMailStore } from '../stores/mail'
import { useMembersStore } from '../stores/members'
import {
  UiButton,
  UiChip,
  UiEmptyState,
  UiErrorState,
  UiIconButton,
  UiPage,
  UiSkeleton,
  UiTextField,
  UiTopAppBar,
} from '../ui'

const mail = useMailStore()
const household = useHouseholdStore()
const members = useMembersStore()
const router = useRouter()

onMounted(() => {
  void members.load()
  void mail.loadFilters()
  void mail.load()
})
watch(
  () => [
    mail.filters.category,
    mail.filters.member,
    mail.filters.source,
    mail.filters.includeIgnored,
  ],
  () => void mail.load(),
)

/** Single choice per row: tapping the selected chip clears it. */
function pick(key: 'category' | 'member' | 'source', value: string) {
  mail.filters[key] = mail.filters[key] === value ? '' : value
}
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar title="Mail">
        <template #actions>
          <UiIconButton icon="label" label="Label mail" @click="router.push('/mail/label')" />
        </template>
      </UiTopAppBar>
    </template>

    <form class="search" role="search" @submit.prevent="mail.load()">
      <UiTextField v-model="mail.filters.q" label="Search" type="search" />
      <UiButton type="submit" variant="tonal" icon="search">Search</UiButton>
    </form>

    <div class="chips" role="group" aria-label="Category">
      <UiChip
        v-for="c in mail.categories"
        :key="c.id"
        variant="filter"
        :selected="mail.filters.category === c.key"
        @click="pick('category', c.key)"
        >{{ c.label }}</UiChip
      >
    </div>
    <div class="chips" role="group" aria-label="Member">
      <UiChip
        v-for="m in members.items"
        :key="m.id"
        variant="filter"
        :selected="mail.filters.member === m.id"
        @click="pick('member', m.id)"
        >{{ m.display_name }}</UiChip
      >
    </div>
    <div class="chips" role="group" aria-label="Source">
      <UiChip
        v-for="s in mail.sources"
        :key="s.id"
        variant="filter"
        :selected="mail.filters.source === s.id"
        @click="pick('source', s.id)"
        >{{ s.name }}</UiChip
      >
      <UiChip
        variant="filter"
        :selected="mail.filters.includeIgnored"
        @click="mail.filters.includeIgnored = !mail.filters.includeIgnored"
        >Show ignored categories</UiChip
      >
    </div>

    <UiSkeleton v-if="mail.loading && !mail.items.length" :lines="5" />
    <UiErrorState v-else-if="mail.error" :message="mail.error" @retry="mail.load()" />
    <UiEmptyState
      v-else-if="!mail.loading && !mail.items.length"
      icon="mail"
      title="No messages."
    />

    <ul class="stack">
      <MessageRow
        v-for="m in mail.items"
        :key="m.id"
        :msg="m"
        :tz="household.tz"
        :categories="mail.categories"
        :members="members.items"
      />
    </ul>
    <UiButton v-if="mail.nextBefore" variant="text" :loading="mail.loading" @click="mail.more()">
      Load more
    </UiButton>
  </UiPage>
</template>

<style scoped>
.search {
  display: flex;
  gap: var(--ui-space-3);
  align-items: flex-start;
}
.search > :first-child {
  flex: 1;
}
.chips {
  display: flex;
  flex-wrap: nowrap;
  gap: var(--ui-space-2);
  overflow-x: auto;
  scrollbar-width: none;
}
.stack {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--ui-space-3);
  margin: 0;
  padding: 0;
}
@media (min-width: 840px) {
  .stack {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
