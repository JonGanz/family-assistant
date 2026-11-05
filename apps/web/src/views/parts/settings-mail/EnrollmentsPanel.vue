<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import type { Member } from '../../../api/client'
import type { EnrollmentView, OrgView } from '../../../api/mailSettings'
import { useOrgsStore } from '../../../stores/orgs'
import ResponsiveItems from '../../../app/ResponsiveItems.vue'
import { UiButton, UiCheckbox, UiSelect, UiTextField } from '../../../ui'
import { describeError } from '../../shared'
import ConfirmDialog from './ConfirmDialog.vue'
import SheetForm from './SheetForm.vue'
import { toggleIn } from './toggle'
import { useConfirm } from './useConfirm'

const props = defineProps<{ child: Member; orgs: OrgView[] }>()
const store = useOrgsStore()
const error = ref('')
const confirm = useConfirm()
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

const remove = (e: EnrollmentView) =>
  confirm.ask(
    'Remove this enrollment?',
    `${orgName(e.org)} for ${props.child.display_name}.`,
    async () => {
      try {
        await store.removeEnrollment(e)
      } catch (err) {
        error.value = describeError(err, 'Could not remove.')
      }
    },
    'Remove',
  )
const orgOptions = () => props.orgs.map((o) => ({ value: o.id, label: o.name }))
</script>

<template>
  <div class="enr" :data-testid="`enrollments-${child.display_name}`">
    <h3>{{ child.display_name }}</h3>
    <ResponsiveItems :items="list()">
      <template #item="{ item: e }">
        <div class="enr__item">
          <strong>{{ orgName(e.org) }}</strong>
          <p class="meta">
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
          </p>
          <div class="row">
            <UiButton variant="text" @click="open(e)">Edit</UiButton>
            <UiButton variant="text" @click="remove(e)">Remove</UiButton>
          </div>
        </div>
      </template>
    </ResponsiveItems>
    <div v-if="orgs.length">
      <UiButton variant="tonal" icon="add" @click="open('new')">Add enrollment</UiButton>
    </div>
    <SheetForm
      :model-value="editing !== null"
      :title="`Enrollment for ${child.display_name}`"
      @update:model-value="editing = null"
      @save="save"
    >
      <UiSelect v-model="form.org" label="Organization" :options="orgOptions()" />
      <UiTextField v-model="form.grade" label="Grade" name="grade" />
      <UiTextField v-model="form.class_name" label="Class" name="class_name" />
      <UiTextField v-model="form.from" label="From" name="from" type="date" />
      <UiTextField v-model="form.until" label="Until" name="until" type="date" />
      <fieldset v-if="contactsOf(form.org).length" class="enr__contacts">
        <legend>Contacts (teachers, coaches)</legend>
        <UiCheckbox
          v-for="c in contactsOf(form.org)"
          :key="c.id"
          :model-value="form.contacts.includes(c.id)"
          :label="`${c.name} (${c.email})`"
          @update:model-value="toggleIn(form.contacts, c.id, $event)"
        />
      </fieldset>
      <p v-if="error" role="alert">{{ error }}</p>
    </SheetForm>
    <ConfirmDialog
      :pending="confirm.pending.value"
      @accept="confirm.accept"
      @cancel="confirm.cancel"
    />
  </div>
</template>

<style scoped>
.enr,
.enr__item {
  display: grid;
  gap: var(--ui-space-2);
}
.enr h3 {
  margin: 0;
  font: var(--ui-type-title-md);
}
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.row {
  display: flex;
  gap: var(--ui-space-2);
}
.enr__contacts {
  display: grid;
  gap: var(--ui-space-1);
  border: 0;
  padding: 0;
  margin: 0;
}
</style>
