<script setup lang="ts">
// Apply a template (Frontend.md §3f, D-120): pick it and a start date, preview the slots it fills, choose Replace or
// Add for every slot that already holds entries, then apply as one change.
import { computed, ref, watch } from 'vue'
import type { MealPlanEntryView, TemplatePreview, TemplateTarget } from '../../../api/food'
import { useMealPlanStore } from '../../../stores/mealPlan'
import { useMealsSettingsStore } from '../../../stores/mealsSettings'
import { UiButton, UiSelect, UiSheet, UiTextField } from '../../../ui'
import { dayLabel, entryTitle } from './planModel'

const props = defineProps<{
  modelValue: boolean
  /** The start date to propose (`YYYY-MM-DD`). */
  defaultStart: string
  minDate: string
  maxDate?: string
  saving?: boolean
  error?: string | null
}>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  apply: [
    templateId: string,
    body: { start: string; add_slots: { date: string; slot_id: string }[] },
  ]
}>()

const store = useMealPlanStore()
const settings = useMealsSettingsStore()

const templateId = ref('')
const start = ref(props.defaultStart)
const preview = ref<TemplatePreview | null>(null)
const busy = ref(false)
const problem = ref<string | null>(null)
/** `replace` or `add` per `date|slot` of a conflicting target. */
const choice = ref<Record<string, 'replace' | 'add'>>({})

const tkey = (t: Pick<TemplateTarget, 'date' | 'slot_id'>) => `${t.date}|${t.slot_id}`
const slotName = (id: string) => settings.slots.find((s) => s.id === id)?.name ?? 'Slot'
/** Entries that hold free text default to Add: Replace would lose a note (D-120). */
const hasFreeText = (l: MealPlanEntryView[]) => l.some((e) => !e.recipe_id)

watch(
  () => props.modelValue,
  async (open) => {
    if (!open) return
    start.value = props.defaultStart
    preview.value = null
    problem.value = null
    if (!store.templatesLoaded) await store.loadTemplates()
    if (!templateId.value || !store.templates.some((t) => t.id === templateId.value))
      templateId.value = store.templates[0]?.id ?? ''
  },
  { immediate: true },
)
// A changed template or date makes the preview stale.
watch([templateId, start], () => (preview.value = null))

const startOk = computed(
  () =>
    /^\d{4}-\d{2}-\d{2}$/.test(start.value) &&
    start.value >= props.minDate &&
    (!props.maxDate || start.value <= props.maxDate),
)

async function show() {
  busy.value = true
  problem.value = null
  try {
    const p = await store.preview(templateId.value, start.value)
    choice.value = Object.fromEntries(
      p.targets
        .filter((t) => t.existing.length > 0)
        .map((t) => [tkey(t), hasFreeText(t.existing) ? 'add' : 'replace'] as const),
    )
    preview.value = p
  } catch (e) {
    problem.value = e instanceof Error ? e.message : 'Could not preview the template'
  } finally {
    busy.value = false
  }
}

const conflicts = computed(() => preview.value?.targets.filter((t) => t.existing.length > 0) ?? [])
function confirm() {
  if (!preview.value) return
  emit('apply', templateId.value, {
    start: start.value,
    add_slots: conflicts.value
      .filter((t) => choice.value[tkey(t)] === 'add')
      .map((t) => ({ date: t.date, slot_id: t.slot_id })),
  })
}
const titles = (t: TemplateTarget) => t.items.map((i) => i.title || i.free_text || '').join(', ')
</script>

<template>
  <UiSheet
    :model-value="modelValue"
    title="Apply a template"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <div class="apply" data-testid="apply-sheet">
      <p v-if="store.templatesLoaded && store.templates.length === 0" class="apply__note">
        There are no templates yet. Make one from the Templates tab or with Save as template.
      </p>
      <template v-else>
        <UiSelect
          v-model="templateId"
          label="Template"
          :options="
            store.templates.map((t) => ({ value: t.id, label: `${t.name} (${t.days} days)` }))
          "
          data-testid="apply-template"
        />
        <UiTextField
          v-model="start"
          type="date"
          label="Start date"
          :error="startOk ? undefined : 'Pick a day inside the plan'"
          data-testid="apply-start"
        />
        <UiButton
          v-if="!preview"
          variant="tonal"
          :disabled="!templateId || !startOk"
          :loading="busy"
          data-testid="apply-preview"
          @click="show"
        >
          Preview
        </UiButton>
      </template>
      <p v-if="problem || error" class="apply__error" role="alert">{{ problem || error }}</p>

      <template v-if="preview">
        <p class="apply__lead" data-testid="apply-summary">
          Fills {{ preview.targets.filter((t) => t.items.length).length }} slots,
          {{ conflicts.length }} already {{ conflicts.length === 1 ? 'has' : 'have' }} entries.
        </p>
        <ul class="apply__targets">
          <li
            v-for="t in preview.targets.filter((x) => x.items.length)"
            :key="tkey(t)"
            class="target"
          >
            <p class="target__head">
              {{ dayLabel(t.date, true) }}, {{ slotName(t.slot_id) }}
              <span v-if="!t.slot_visible" class="target__hidden">(hidden slot)</span>
            </p>
            <p class="target__items">{{ titles(t) }}</p>
            <fieldset
              v-if="t.existing.length"
              class="conflict"
              :data-testid="`template-conflict-${t.existing[0]!.id}`"
            >
              <legend>Already planned: {{ t.existing.map(entryTitle).join(', ') }}</legend>
              <label class="conflict__opt">
                <input
                  v-model="choice[tkey(t)]"
                  type="radio"
                  :name="`c-${tkey(t)}`"
                  value="replace"
                  :data-testid="`template-replace-${t.existing[0]!.id}`"
                />
                Replace
              </label>
              <label class="conflict__opt">
                <input
                  v-model="choice[tkey(t)]"
                  type="radio"
                  :name="`c-${tkey(t)}`"
                  value="add"
                  :data-testid="`template-add-${t.existing[0]!.id}`"
                />
                Add
              </label>
            </fieldset>
          </li>
        </ul>
      </template>
    </div>
    <template #actions>
      <UiButton variant="text" @click="emit('update:modelValue', false)">Cancel</UiButton>
      <UiButton
        v-if="preview"
        :loading="saving"
        data-testid="template-apply-confirm"
        @click="confirm"
      >
        Apply
      </UiButton>
    </template>
  </UiSheet>
</template>

<style scoped>
.apply {
  display: grid;
  gap: var(--ui-space-3);
}
.apply__note,
.apply__lead {
  margin: 0;
}
.apply__error {
  margin: 0;
  color: var(--ui-error);
}
.apply__targets {
  display: grid;
  gap: var(--ui-space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}
.target {
  padding: var(--ui-space-2) var(--ui-space-3);
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-sm);
}
.target__head {
  margin: 0;
  font: var(--ui-type-label-lg);
}
.target__hidden {
  color: var(--ui-on-surface-variant);
}
.target__items {
  margin: 0;
  font: var(--ui-type-body-md);
}
.conflict {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-4);
  margin: var(--ui-space-2) 0 0;
  padding: 0;
  border: 0;
}
.conflict legend {
  padding: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.conflict__opt {
  display: inline-flex;
  align-items: center;
  gap: var(--ui-space-2);
  min-height: var(--ui-touch);
}
</style>
