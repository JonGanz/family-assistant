<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import type { Member } from '../../../api/client'
import type { EnrollmentView, OrgView } from '../../../api/mailSettings'
import { useOrgsStore } from '../../../stores/orgs'
import { describeError } from '../../shared'

const props = defineProps<{ child: Member; orgs: OrgView[] }>()
const store = useOrgsStore()
const error = ref('')
const editing = ref<string | 'new' | null>(null)
const form = reactive({
  org: '',
  grade: '',
  class_name: '',
  from: '',
  until: '',
  contacts: [] as string[],
})

onMounted(() => store.loadEnrollments(props.child.id).catch(() => undefined))

const list = () => store.enrollments[props.child.id] ?? []
const orgName = (id: string) => props.orgs.find((o) => o.id === id)?.name ?? '?'
const contactsOf = (org: string) => props.orgs.find((o) => o.id === org)?.contacts ?? []

function open(e: EnrollmentView | 'new') {
  error.value = ''
  editing.value = e === 'new' ? 'new' : e.id
  form.org = e === 'new' ? (props.orgs[0]?.id ?? '') : e.org
  form.grade = e === 'new' ? '' : (e.grade ?? '')
  form.class_name = e === 'new' ? '' : (e.class_name ?? '')
  form.from = e === 'new' ? '' : (e.from ?? '')
  form.until = e === 'new' ? '' : (e.until ?? '')
  form.contacts = e === 'new' ? [] : [...e.contacts]
}

async function save() {
  error.value = ''
  const body = {
    org: form.org,
    grade: form.grade.trim() || null,
    class_name: form.class_name.trim() || null,
    from: form.from || null,
    until: form.until || null,
    contacts: form.contacts,
  }
  try {
    const cur = list().find((x) => x.id === editing.value)
    const ok = cur
      ? await store.patchEnrollment(cur, body)
      : await store.addEnrollment(props.child.id, body)
    if (ok) editing.value = null
  } catch (e) {
    error.value = describeError(e, 'Could not save the enrollment.')
  }
}

async function remove(e: EnrollmentView) {
  if (!confirm('Remove this enrollment?')) return
  try {
    await store.removeEnrollment(e)
  } catch (err) {
    error.value = describeError(err, 'Could not remove.')
  }
}
</script>

<template>
  <div :data-testid="`enrollments-${child.display_name}`">
    <h3>{{ child.display_name }}</h3>
    <ul class="settings-list">
      <li v-for="e in list()" :key="e.id">
        <strong>{{ orgName(e.org) }}</strong>
        <span class="settings-note">
          {{
            [
              e.grade && `grade ${e.grade}`,
              e.class_name,
              e.from && `from ${e.from}`,
              e.until && `until ${e.until}`,
            ]
              .filter(Boolean)
              .join(' · ')
          }}
        </span>
        <button type="button" @click="open(e)">Edit</button>
        <button type="button" @click="remove(e)">Remove</button>
      </li>
    </ul>
    <button v-if="editing === null && orgs.length" type="button" @click="open('new')">
      Add enrollment
    </button>
    <form v-if="editing !== null" class="settings-form" @submit.prevent="save">
      <label>
        Organization
        <select v-model="form.org" name="org" required>
          <option v-for="o in orgs" :key="o.id" :value="o.id">{{ o.name }}</option>
        </select>
      </label>
      <label>Grade <input v-model="form.grade" name="grade" /></label>
      <label>Class <input v-model="form.class_name" name="class_name" /></label>
      <label>From <input v-model="form.from" name="from" type="date" /></label>
      <label>Until <input v-model="form.until" name="until" type="date" /></label>
      <fieldset v-if="contactsOf(form.org).length">
        <legend>Contacts (teachers, coaches)</legend>
        <label v-for="c in contactsOf(form.org)" :key="c.id" class="inline">
          <input v-model="form.contacts" type="checkbox" :value="c.id" />
          {{ c.name }} ({{ c.email }})
        </label>
      </fieldset>
      <p v-if="error" class="settings-error" role="alert">{{ error }}</p>
      <div>
        <button type="submit">Save</button>
        <button type="button" @click="editing = null">Cancel</button>
      </div>
    </form>
  </div>
</template>
