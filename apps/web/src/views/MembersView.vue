<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import type { Member } from '../api/client'
import { ApiError } from '../api/http'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'

const session = useSessionStore()
const members = useMembersStore()

const editing = ref<string | null>(null)
const form = reactive({ display_name: '', color: '', avatar: '', grade: '', interests: '' })
const creating = reactive({ display_name: '', role: 'child' })
const error = ref('')

const csv = (s: string) =>
  s
    .split(',')
    .map((x) => x.trim())
    .filter(Boolean)

onMounted(() => members.load())

function startEdit(m: Member) {
  editing.value = m.id
  members.conflictId = null
  error.value = ''
  form.display_name = m.display_name
  form.color = m.color ?? ''
  form.avatar = m.avatar ?? ''
  form.grade = m.grade ?? ''
  form.interests = (m.interests ?? []).join(', ')
}

async function save(m: Member) {
  error.value = ''
  try {
    const ok = await members.update(m, {
      display_name: form.display_name,
      color: form.color || null,
      avatar: form.avatar || null,
      grade: form.grade || null,
      interests: csv(form.interests),
    })
    if (ok) editing.value = null
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Save failed.'
  }
}

async function reload() {
  await members.load()
  const m = members.items.find((x) => x.id === editing.value)
  if (m) startEdit(m)
}

async function create() {
  error.value = ''
  try {
    await members.create({ display_name: creating.display_name, role: creating.role })
    creating.display_name = ''
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Create failed.'
  }
}
</script>

<template>
  <section>
    <h1>Members</h1>
    <p v-if="members.error" class="error" role="alert">{{ members.error }}</p>
    <ul class="members">
      <li v-for="m in members.items" :key="m.id" :data-testid="`member-${m.display_name}`">
        <template v-if="editing !== m.id">
          <span class="swatch" :style="{ background: m.color ?? 'var(--color-border)' }">
            {{ m.avatar || m.display_name.slice(0, 1) }}
          </span>
          <strong>{{ m.display_name }}</strong>
          <em>{{ m.role }}</em>
          <button v-if="session.isAdult && m.version !== undefined" @click="startEdit(m)">
            Edit
          </button>
        </template>
        <form v-else @submit.prevent="save(m)">
          <label>Name <input v-model="form.display_name" name="display_name" required /></label>
          <label>Colour <input v-model="form.color" name="color" placeholder="#2f6fed" /></label>
          <label>Avatar <input v-model="form.avatar" name="avatar" maxlength="4" /></label>
          <label>Grade <input v-model="form.grade" name="grade" /></label>
          <label>
            Interests (comma separated)
            <input v-model="form.interests" name="interests" />
          </label>
          <p v-if="members.conflictId === m.id" class="error" role="alert">
            Someone else changed this member.
            <button type="button" @click="reload">Reload</button>
          </p>
          <p v-if="error" class="error" role="alert">{{ error }}</p>
          <button type="submit">Save</button>
          <button type="button" @click="editing = null">Cancel</button>
        </form>
      </li>
    </ul>

    <form v-if="session.isAdult" class="create" @submit.prevent="create">
      <h2>Add member</h2>
      <label>Name <input v-model="creating.display_name" name="new_name" required /></label>
      <label>
        Role
        <select v-model="creating.role" name="new_role">
          <option value="child">child</option>
          <option value="adult">adult</option>
        </select>
      </label>
      <button type="submit">Add</button>
    </form>
  </section>
</template>

<style scoped>
.members {
  list-style: none;
  padding: 0;
  display: grid;
  gap: 0.5rem;
}
.members li {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  padding: 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: 0.5rem;
}
.swatch {
  width: 2.25rem;
  height: 2.25rem;
  border-radius: 50%;
  display: grid;
  place-items: center;
  color: #fff;
}
form {
  display: grid;
  gap: 0.5rem;
  width: 100%;
}
label {
  display: grid;
  gap: 0.15rem;
}
.error {
  color: #c0392b;
}
.create {
  margin-top: 2rem;
  max-width: 22rem;
}
</style>
