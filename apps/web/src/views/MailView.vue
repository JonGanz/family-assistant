<script setup lang="ts">
import PageHeader from '../components/PageHeader.vue'
import StateBlock from '../components/StateBlock.vue'
import { onMounted, watch } from 'vue'
import MessageRow from '../components/mail/MessageRow.vue'
import { useHouseholdStore } from '../stores/household'
import { useMailStore } from '../stores/mail'
import { useMembersStore } from '../stores/members'

const mail = useMailStore()
const household = useHouseholdStore()
const members = useMembersStore()

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
</script>

<template>
  <section aria-labelledby="page-title">
    <PageHeader title="Mail">
      <template #actions><RouterLink to="/mail/label">Label mail</RouterLink></template>
    </PageHeader>
    <form class="form form-wide filters" @submit.prevent="mail.load()">
      <label>
        Search
        <input v-model="mail.filters.q" type="search" placeholder="Sender or subject" />
      </label>
      <label>
        Category
        <select v-model="mail.filters.category">
          <option value="">All</option>
          <option v-for="c in mail.categories" :key="c.id" :value="c.key">{{ c.label }}</option>
        </select>
      </label>
      <label>
        Member
        <select v-model="mail.filters.member">
          <option value="">Everyone</option>
          <option v-for="m in members.items" :key="m.id" :value="m.id">{{ m.display_name }}</option>
        </select>
      </label>
      <label>
        Source
        <select v-model="mail.filters.source">
          <option value="">All</option>
          <option v-for="s in mail.sources" :key="s.id" :value="s.id">{{ s.name }}</option>
        </select>
      </label>
      <label class="field-inline">
        <input v-model="mail.filters.includeIgnored" type="checkbox" /> Show ignored categories
      </label>
      <div>
        <button type="submit" class="btn-primary">Search</button>
      </div>
    </form>
    <StateBlock
      :loading="mail.loading && !mail.items.length"
      :error="mail.error"
      :empty="!mail.loading && !mail.items.length"
      empty-text="No messages."
    />
    <ul class="card-list plain">
      <li v-for="m in mail.items" :key="m.id">
        <MessageRow
          :msg="m"
          :tz="household.tz"
          :categories="mail.categories"
          :members="members.items"
        />
      </li>
    </ul>
    <button v-if="mail.nextBefore" type="button" :disabled="mail.loading" @click="mail.more()">
      Load more
    </button>
  </section>
</template>

<style scoped>
.filters {
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  align-items: end;
  margin-bottom: var(--space-3);
}
.filters > label:not(.field-inline) {
  display: grid;
  gap: var(--space-1);
}
.plain > li {
  padding: 0;
  border: 0;
}
</style>
