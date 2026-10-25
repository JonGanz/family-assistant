<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { useAwayStore } from '../../stores/away'
import { useMembersStore } from '../../stores/members'
import { describeError } from '../shared'
import './settings.css'

const away = useAwayStore()
const members = useMembersStore()
const form = reactive({ member: '', from: '', until: '', note: '' })
const error = ref('')

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
  <div>
    <p class="settings-note">
      While someone is away they are skipped when tasks are handed out for those days.
    </p>
    <p v-if="away.error" class="settings-error" role="alert">{{ away.error }}</p>
    <ul class="settings-list">
      <template v-for="m in members.items" :key="m.id">
        <li v-for="a in away.byMember[m.id] ?? []" :key="a.id">
          <strong>{{ nameOf(a.member) }}</strong> {{ a.from }} to {{ a.until }}
          <span v-if="a.note" class="settings-note"> · {{ a.note }}</span>
          <button type="button" @click="remove(a.member, a.id)">Remove</button>
        </li>
      </template>
    </ul>
    <form class="settings-form" @submit.prevent="add">
      <label>
        Who
        <select v-model="form.member" name="member" required>
          <option v-for="m in members.items" :key="m.id" :value="m.id">{{ m.display_name }}</option>
        </select>
      </label>
      <label>From <input v-model="form.from" name="from" type="date" required /></label>
      <label
        >Until (last day) <input v-model="form.until" name="until" type="date" required
      /></label>
      <label>Note <input v-model="form.note" name="note" /></label>
      <p v-if="error" class="settings-error" role="alert">{{ error }}</p>
      <button type="submit">Add absence</button>
    </form>
  </div>
</template>
