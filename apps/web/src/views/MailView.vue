<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
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
  UiFilterChips,
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

const categoryOptions = computed(() =>
  mail.categories.map((c) => ({ value: c.key, label: c.label })),
)
const memberOptions = computed(() =>
  members.items.map((m) => ({ value: m.id, label: m.display_name })),
)
const sourceOptions = computed(() => mail.sources.map((s) => ({ value: s.id, label: s.name })))

onMounted(() => {
  void members.load()
  void mail.loadFilters()
  void mail.load()
})
watch(
  () => [
    mail.filters.category.join(),
    mail.filters.member.join(),
    mail.filters.source.join(),
    mail.filters.includeIgnored,
  ],
  () => void mail.load(),
)
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

    <UiFilterChips v-model="mail.filters.category" :options="categoryOptions" label="Category" />
    <UiFilterChips v-model="mail.filters.member" :options="memberOptions" label="Member" />
    <div class="chips">
      <UiFilterChips v-model="mail.filters.source" :options="sourceOptions" label="Source" />
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
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ui-space-2);
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
