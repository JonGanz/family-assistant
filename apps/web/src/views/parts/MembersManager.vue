<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import MemberAccess from './MemberAccess.vue'
import { validateNewMember } from './memberForm'
import { api, type Member, type PatchMemberReq } from '../../api/client'
import type { Gender } from '../../api/mailSettings'
import ChipInput from './settings-mail/ChipInput.vue'
import HuePicker from '../../components/HuePicker.vue'
import Avatar from '../../components/Avatar.vue'
import ErrorBanner from '../../components/ErrorBanner.vue'
import StateBlock from '../../components/StateBlock.vue'
import { hueVar, type Hue } from '../../lib/hues'
import { describeError } from '../shared'
import { useMembersStore } from '../../stores/members'
import { useSessionStore } from '../../stores/session'

const members = useMembersStore()

const editing = ref<string | null>(null)
const form = reactive({
  display_name: '',
  color: 'blue' as Hue,
  avatar: '',
  grade: '',
  interests: '',
  hobbies: '',
  responsibilities: '',
  aliases: [] as string[],
  gender: '' as Gender | '',
  tags: [] as string[],
})
const preview = ref<string | null>(null)
const pending = ref<File | null>(null)
const creating = reactive({
  display_name: '',
  role: 'child',
  color: 'blue' as Hue,
  grade: '',
  aliases: [] as string[],
  username: '',
  password: '',
  pin: '',
})
const createErrors = ref<Record<string, string>>({})
const error = ref('')
const createdNote = ref('')

const session = useSessionStore()
/** A claimed adult other than the signed-in one cannot be edited (D-006). */
const claimedByOther = (m: Member) =>
  m.role === 'adult' && m.id !== session.member?.id && m.has_account && !m.initial_password

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
  form.color = m.color ?? 'blue'
  clearPending()
  form.avatar = m.avatar ?? ''
  form.grade = m.grade ?? ''
  form.interests = (m.interests ?? []).join(', ')
  form.hobbies = (m.hobbies ?? []).join(', ')
  form.responsibilities = (m.responsibilities ?? []).join(', ')
  form.aliases = [...(m.aliases ?? [])]
  form.gender = m.gender ?? ''
  form.tags = [...(m.relevance_tags ?? [])]
}

function clearPending() {
  if (preview.value) URL.revokeObjectURL(preview.value)
  preview.value = null
  pending.value = null
}

function pickFile(ev: Event) {
  const file = (ev.target as HTMLInputElement).files?.[0]
  clearPending()
  if (!file) return
  const problem = members.checkAvatar(file)
  if (problem) {
    error.value = problem
    ;(ev.target as HTMLInputElement).value = ''
    return
  }
  error.value = ''
  pending.value = file
  preview.value = URL.createObjectURL(file)
}

async function save(m: Member) {
  error.value = ''
  try {
    const ok = await members.update(m, {
      display_name: form.display_name,
      color: form.color,
      avatar: form.avatar || null,
      grade: m.role === 'child' ? form.grade || null : null,
      interests: csv(form.interests),
      hobbies: csv(form.hobbies),
      responsibilities: csv(form.responsibilities),
      // M3 fields; the generated patch type does not list them yet.
      aliases: form.aliases,
      ...(m.role === 'child' ? { gender: form.gender || null, relevance_tags: form.tags } : {}),
    } as PatchMemberReq)
    if (ok) {
      if (pending.value) await members.uploadAvatar(m.id, pending.value)
      clearPending()
      editing.value = null
    }
  } catch (e) {
    error.value = describeError(e, 'Save failed.')
  }
}

async function removeAvatar(m: Member) {
  error.value = ''
  try {
    await members.removeAvatar(m.id)
  } catch (e) {
    error.value = describeError(e, 'Could not remove the picture.')
  }
}

async function reload() {
  await members.load()
  const m = members.items.find((x) => x.id === editing.value)
  if (m) startEdit(m)
}

async function create() {
  error.value = ''
  createdNote.value = ''
  createErrors.value = validateNewMember({
    ...creating,
    pin: creating.role === 'adult' ? '' : creating.pin,
  })
  if (Object.keys(createErrors.value).length) return
  try {
    const made = await members.create({
      display_name: creating.display_name.trim(),
      role: creating.role,
      color: creating.color,
      grade: creating.role === 'child' ? creating.grade.trim() || null : null,
      aliases: creating.aliases,
    })
    const user = creating.username.trim()
    if (user) await api.createAccount(made.id, user, creating.password)
    if (creating.pin && creating.role === 'child') await api.setPin(made.id, creating.pin)
    await members.load()
    createdNote.value =
      user && creating.role === 'adult' ? 'They must change this password at first sign-in.' : ''
    if (!createdNote.value)
      Object.assign(creating, {
        display_name: '',
        grade: '',
        aliases: [],
        username: '',
        password: '',
        pin: '',
      })
  } catch (e) {
    error.value = describeError(e, 'Create failed.')
    await members.load()
  }
}
</script>

<template>
  <div class="members-manager">
    <StateBlock
      :error="members.error"
      :loading="members.loading && !members.items.length"
      :empty="!members.loading && !members.items.length"
      empty-text="No members yet."
    />
    <ul class="members card-list">
      <li v-for="m in members.items" :key="m.id" :data-testid="`member-${m.display_name}`">
        <template v-if="editing !== m.id">
          <Avatar
            :name="m.display_name"
            :color="hueVar(m.color)"
            :src="m.avatar_url"
            size="2.5rem"
          />
          <strong>{{ m.display_name }}</strong>
          <em>{{ m.role }}</em>
          <button
            v-if="m.version !== undefined && !claimedByOther(m)"
            class="btn-sm"
            @click="startEdit(m)"
          >
            Edit
          </button>
          <span v-else-if="claimedByOther(m)" class="muted">Manages their own profile</span>
        </template>
        <form v-else class="form form-wide" @submit.prevent="save(m)">
          <label class="field"
            >Name <input v-model="form.display_name" name="display_name" required
          /></label>
          <HuePicker v-model="form.color" name="color" />
          <div class="avatar-edit">
            <Avatar
              :name="form.display_name || m.display_name"
              :color="hueVar(form.color)"
              :src="preview ?? m.avatar_url"
              size="4rem"
            />
            <label class="field">
              Picture (PNG, JPEG or WebP, up to 512 KB)
              <input
                type="file"
                name="avatar_file"
                accept="image/png,image/jpeg,image/webp"
                @change="pickFile"
              />
            </label>
            <button
              v-if="m.avatar_url && !preview"
              type="button"
              class="btn-sm"
              @click="removeAvatar(m)"
            >
              Remove picture
            </button>
          </div>
          <label class="field"
            >Initials or emoji <input v-model="form.avatar" name="avatar" maxlength="4"
          /></label>
          <label v-if="m.role === 'child'" class="field"
            >Grade <input v-model="form.grade" name="grade" />
          </label>
          <label class="field">
            Interests (comma separated)
            <input v-model="form.interests" name="interests" />
          </label>
          <label class="field">
            Hobbies (comma separated)
            <input v-model="form.hobbies" name="hobbies" />
          </label>
          <label class="field">
            Responsibilities (comma separated)
            <input v-model="form.responsibilities" name="responsibilities" />
          </label>
          <ChipInput v-model="form.aliases" label="Also known as (names used in mail)" />
          <template v-if="m.role === 'child'">
            <label class="field">
              Gender (helps match girls' or boys' groups; optional)
              <select v-model="form.gender" name="gender">
                <option value="">Not set</option>
                <option value="female">Female</option>
                <option value="male">Male</option>
                <option value="other">Other</option>
              </select>
            </label>
            <ChipInput
              v-model="form.tags"
              label="Relevance tags (for example a sport or a class)"
            />
          </template>
          <ErrorBanner v-if="members.conflictId === m.id" conflict @reload="reload" />
          <ErrorBanner v-if="error" :message="error" />
          <MemberAccess v-if="m.has_account !== undefined" :member="m" />
          <div class="row">
            <button type="submit" class="btn-primary">Save</button>
            <button type="button" class="btn-ghost" @click="editing = null">Cancel</button>
          </div>
        </form>
      </li>
    </ul>

    <ErrorBanner v-if="error && editing === null" :message="error" />
    <form class="form create" data-testid="add-member" @submit.prevent="create">
      <h2>Add member</h2>
      <label class="field"
        >Name <input v-model="creating.display_name" name="new_name" required
      /></label>
      <p v-if="createErrors.display_name" class="text-danger" role="alert">
        {{ createErrors.display_name }}
      </p>
      <label class="field">
        Role
        <select v-model="creating.role" name="new_role">
          <option value="child">child</option>
          <option value="adult">adult</option>
        </select>
      </label>
      <HuePicker v-model="creating.color" name="new_color" />
      <label v-if="creating.role === 'child'" class="field"
        >Grade (optional) <input v-model="creating.grade" name="new_grade"
      /></label>
      <ChipInput v-model="creating.aliases" label="Also known as (names used in mail)" />
      <fieldset>
        <legend>
          {{ creating.role === 'adult' ? 'Login (optional)' : 'Login and PIN (optional)' }}
        </legend>
        <label class="field"
          >Username <input v-model="creating.username" name="new_username" autocomplete="off"
        /></label>
        <p v-if="createErrors.username" class="text-danger" role="alert">
          {{ createErrors.username }}
        </p>
        <label class="field"
          >Password
          <input
            v-model="creating.password"
            name="new_password"
            type="password"
            autocomplete="new-password"
        /></label>
        <p v-if="createErrors.password" class="text-danger" role="alert">
          {{ createErrors.password }}
        </p>
        <p v-if="creating.role === 'adult'" class="muted">
          The adult sets their own PIN after signing in, and must change this password at first
          sign-in.
        </p>
        <label v-if="creating.role === 'child'" class="field"
          >PIN (4 to 6 digits)
          <input
            v-model="creating.pin"
            name="new_pin"
            inputmode="numeric"
            autocomplete="off"
            maxlength="6"
        /></label>
        <p v-if="createErrors.pin" class="text-danger" role="alert">{{ createErrors.pin }}</p>
      </fieldset>
      <button type="submit" class="btn-primary">Add</button>
      <p v-if="createdNote" class="text-success" role="status" data-testid="created-note">
        {{ createdNote }}
      </p>
    </form>
  </div>
</template>

<style scoped>
.members > li {
  display: flex;
  gap: var(--space-3);
  align-items: center;
}
.avatar-edit {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}
form {
  width: 100%;
}
.create {
  margin-top: 2rem;
  max-width: 22rem;
}
</style>
