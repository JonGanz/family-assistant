<script setup lang="ts">
import { onMounted, watch } from 'vue'
import MessageRow from '../components/mail/MessageRow.vue'
import { useHouseholdStore } from '../stores/household'
import { useMailStore } from '../stores/mail'
import { useMembersStore } from '../stores/members'
import './parts/settings.css'

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
  <section>
    <h1>Mail</h1>
    <form class="settings-form filters" @submit.prevent="mail.load()">
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
      <label class="inline">
        <input v-model="mail.filters.includeIgnored" type="checkbox" /> Show ignored categories
      </label>
      <div>
        <button type="submit">Search</button> <RouterLink to="/mail/label">Label mail</RouterLink>
      </div>
    </form>
    <p v-if="mail.error" class="settings-error" role="alert">{{ mail.error }}</p>
    <p v-if="!mail.loading && !mail.items.length && !mail.error" class="settings-note">
      No messages.
    </p>
    <ul class="settings-list plain">
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
  max-width: none;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  align-items: end;
  margin-bottom: var(--space-3);
}
.plain > li {
  padding: 0;
  border: 0;
}
</style>
