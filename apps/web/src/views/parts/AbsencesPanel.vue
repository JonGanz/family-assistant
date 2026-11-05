<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useAwayStore } from '../../stores/away'
import { useMembersStore } from '../../stores/members'
import ResponsiveItems from '../../app/ResponsiveItems.vue'
import { UiButton, UiEmptyState, UiSelect, UiSheet, UiTextField } from '../../ui'
import { describeError } from '../shared'
import PageFab from './settings-mail/PageFab.vue'

const away = useAwayStore()
const members = useMembersStore()
const form = reactive({ member: '', from: '', until: '', note: '' })
const error = ref('')
const adding = ref(false)
const memberOptions = computed(() =>
  members.items.map((m) => ({ value: m.id, label: m.display_name })),
)
const all = computed(() => members.items.flatMap((m) => away.byMember[m.id] ?? []))

onMounted(async () => {
  await members.load()
  await away.load(members.items.map((m) => m.id))
  form.member = members.items[0]?.id ?? ''
})

const nameOf = (id: string) => members.items.find((m) => m.id === id)?.display_name ?? '?'

async function add() {
  error.value = ''
  if (form.until < form.from) {
    error.value = 'The last day must not be before the first day.'
    return
  }
  try {
    await away.add(form.member, {
      from: form.from,
      until: form.until,
      note: form.note.trim() || null,
    })
    form.from = form.until = form.note = ''
    adding.value = false
  } catch (e) {
    error.value = describeError(e, 'Could not save the absence.')
  }
}
async function remove(member: string, id: string) {
  try {
    await away.remove(member, id)
  } catch (e) {
    error.value = describeError(e, 'Could not remove the absence.')
  }
}
</script>

<template>
  <div class="panel">
    <p class="meta">
      While someone is away they are skipped when tasks are handed out for those days.
    </p>
    <p v-if="away.error" role="alert">{{ away.error }}</p>
    <UiEmptyState v-if="!all.length" icon="calendar" title="Nobody is away." />
    <ResponsiveItems :items="all">
      <template #item="{ item: a }">
        <div class="item">
          <div class="item__text">
            <strong>{{ nameOf(a.member) }}</strong>
            <p class="meta">
              {{ a.from }} to {{ a.until }}<template v-if="a.note"> · {{ a.note }}</template>
            </p>
          </div>
          <UiButton variant="text" @click="remove(a.member, a.id)">Remove</UiButton>
        </div>
      </template>
    </ResponsiveItems>

    <PageFab label="Add absence" @click="adding = true" />
    <UiSheet v-model="adding" title="Add absence">
      <form class="form" @submit.prevent="add">
        <UiSelect v-model="form.member" label="Who" :options="memberOptions" />
        <UiTextField v-model="form.from" label="From" name="from" type="date" required />
        <UiTextField
          v-model="form.until"
          label="Until (last day)"
          name="until"
          type="date"
          required
        />
        <UiTextField v-model="form.note" label="Note" name="note" />
        <p v-if="error" role="alert">{{ error }}</p>
        <div class="row">
          <UiButton variant="text" @click="adding = false">Cancel</UiButton>
          <UiButton type="submit">Save</UiButton>
        </div>
      </form>
    </UiSheet>
  </div>
</template>

<style scoped>
.panel,
.form {
  display: grid;
  gap: var(--ui-space-3);
}
.item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ui-space-2);
}
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.row {
  display: flex;
  justify-content: flex-end;
  gap: var(--ui-space-2);
}
</style>
