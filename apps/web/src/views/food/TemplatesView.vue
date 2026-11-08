<script setup lang="ts">
// Food → Templates (adults): the list with New, Duplicate (asks for a name) and Delete (asks first); D-112, D-122.
import { computed, onMounted, ref } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import type { MealTemplateSummary } from '../../api/food'
import FoodTabs from '../../components/food/FoodTabs.vue'
import { useMealPlanStore } from '../../stores/mealPlan'
import { useMealsSettingsStore } from '../../stores/mealsSettings'
import {
  UiButton,
  UiCard,
  UiDialog,
  UiEmptyState,
  UiErrorState,
  UiPage,
  UiSkeleton,
  UiTextField,
  UiTopAppBar,
  useSnackbar,
} from '../../ui'
import { describeError } from '../shared'

const store = useMealPlanStore()
const settings = useMealsSettingsStore()
const snackbar = useSnackbar()
const router = useRouter()

onMounted(() => {
  void store.loadTemplates()
  if (!settings.loaded) void settings.load()
})

const slotNames = (t: MealTemplateSummary) =>
  t.slot_ids.map((id) => settings.slots.find((s) => s.id === id)?.name ?? 'Slot').join(', ')

const dupOf = ref<MealTemplateSummary | null>(null)
const dupName = ref('')
const dupBusy = ref(false)
const dupOpen = computed({
  get: () => dupOf.value !== null,
  set: (v) => {
    if (!v) dupOf.value = null
  },
})
function askDuplicate(t: MealTemplateSummary) {
  dupName.value = `${t.name} copy`
  dupOf.value = t
}
async function duplicate() {
  const t = dupOf.value
  if (!t || !dupName.value.trim()) return
  dupBusy.value = true
  try {
    const v = await store.duplicateTemplate(t.id, dupName.value.trim())
    dupOf.value = null
    await router.push(`/food/templates/${v.id}`)
  } catch (e) {
    snackbar.show({ text: describeError(e) })
  } finally {
    dupBusy.value = false
  }
}

const delOf = ref<MealTemplateSummary | null>(null)
const delOpen = computed({
  get: () => delOf.value !== null,
  set: (v) => {
    if (!v) delOf.value = null
  },
})
async function remove() {
  const t = delOf.value
  if (!t) return
  delOf.value = null
  try {
    await store.deleteTemplate(t.id)
    snackbar.show({ text: `Deleted ${t.name}` })
  } catch (e) {
    snackbar.show({ text: describeError(e) })
  }
}
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar title="Food" />
    </template>

    <FoodTabs />

    <div class="tpl-bar">
      <UiButton icon="add" to="/food/templates/new" data-testid="template-new"
        >New template</UiButton
      >
    </div>

    <UiSkeleton v-if="!store.templatesLoaded && !store.error" :lines="4" />
    <UiErrorState
      v-else-if="store.error && !store.templatesLoaded"
      :message="store.error"
      @retry="store.loadTemplates()"
    />
    <UiEmptyState
      v-else-if="store.templates.length === 0"
      icon="template"
      title="No templates yet"
      text="A template is a reusable run of days with meals in slots. Make one here or save a stretch of the plan."
    />
    <ul v-else class="tpl-list">
      <li v-for="t in store.templates" :key="t.id">
        <UiCard>
          <div class="tpl" :data-testid="`template-${t.id}`">
            <RouterLink class="tpl__main" :to="`/food/templates/${t.id}`">
              <strong>{{ t.name }}</strong>
              <span class="tpl__meta">
                {{ t.days }} {{ t.days === 1 ? 'day' : 'days'
                }}<template v-if="t.slot_ids.length"> · {{ slotNames(t) }}</template>
              </span>
            </RouterLink>
            <UiButton
              variant="text"
              icon="copy"
              :data-testid="`template-duplicate-${t.id}`"
              @click="askDuplicate(t)"
            >
              Duplicate
            </UiButton>
            <UiButton
              variant="text"
              icon="delete"
              :data-testid="`template-delete-${t.id}`"
              @click="delOf = t"
            >
              Delete
            </UiButton>
          </div>
        </UiCard>
      </li>
    </ul>

    <UiDialog v-model="dupOpen" title="Duplicate template">
      <UiTextField v-model="dupName" label="Name of the copy" data-testid="template-dup-name" />
      <template #actions>
        <UiButton variant="text" @click="dupOpen = false">Cancel</UiButton>
        <UiButton
          :disabled="!dupName.trim()"
          :loading="dupBusy"
          data-testid="template-dup-confirm"
          @click="duplicate"
        >
          Duplicate
        </UiButton>
      </template>
    </UiDialog>
    <UiDialog v-model="delOpen" title="Delete template?">
      <p>Delete "{{ delOf?.name }}"? Entries already applied to the plan stay where they are.</p>
      <template #actions>
        <UiButton variant="text" @click="delOpen = false">Cancel</UiButton>
        <UiButton data-testid="template-delete-confirm" @click="remove">Delete</UiButton>
      </template>
    </UiDialog>
  </UiPage>
</template>

<style scoped>
.tpl-list {
  display: grid;
  gap: var(--ui-space-3);
  margin: 0;
  padding: 0;
  list-style: none;
}
.tpl {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ui-space-2);
}
.tpl__main {
  display: grid;
  flex: 1;
  min-width: 10rem;
  color: inherit;
  text-decoration: none;
}
.tpl__meta {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
</style>
