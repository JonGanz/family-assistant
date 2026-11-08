<script setup lang="ts">
// Save a plan range as a template (D-122): a name, the dates and the slots to include.
import { computed, ref, watch } from 'vue'
import { useMealsSettingsStore } from '../../../stores/mealsSettings'
import { UiButton, UiCheckbox, UiSheet, UiTextField } from '../../../ui'
import { addDays } from '../../../lib/time'

const props = defineProps<{
  modelValue: boolean
  defaultFrom: string
  saving?: boolean
  error?: string | null
}>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  save: [body: { name: string; from: string; to: string; slot_ids: string[] }]
}>()

const settings = useMealsSettingsStore()
const name = ref('')
const from = ref(props.defaultFrom)
const to = ref(addDays(props.defaultFrom, 6))
const chosen = ref<string[]>([])

watch(
  () => props.modelValue,
  (open) => {
    if (!open) return
    name.value = ''
    from.value = props.defaultFrom
    to.value = addDays(props.defaultFrom, 6)
    chosen.value = settings.visibleSlots.flatMap((s) => (s.id ? [s.id] : []))
  },
  { immediate: true },
)

const isDay = (v: string) => /^\d{4}-\d{2}-\d{2}$/.test(v)
const days = computed(() =>
  isDay(from.value) && isDay(to.value)
    ? Math.round(
        (Date.parse(`${to.value}T00:00:00Z`) - Date.parse(`${from.value}T00:00:00Z`)) / 86_400_000,
      ) + 1
    : 0,
)
const rangeOk = computed(() => days.value >= 1 && days.value <= 366)
const valid = computed(() => name.value.trim() !== '' && rangeOk.value && chosen.value.length > 0)
const toggle = (id: string, on: boolean) =>
  (chosen.value = on ? [...chosen.value, id] : chosen.value.filter((x) => x !== id))

function submit() {
  if (!valid.value) return
  emit('save', {
    name: name.value.trim(),
    from: from.value,
    to: to.value,
    slot_ids: settings.slots
      .flatMap((s) => (s.id ? [s.id] : []))
      .filter((id) => chosen.value.includes(id)),
  })
}
</script>

<template>
  <UiSheet
    :model-value="modelValue"
    title="Save as template"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <form class="save" data-testid="save-template-sheet" @submit.prevent="submit">
      <UiTextField v-model="name" label="Name" required data-testid="save-template-name" />
      <div class="save__dates">
        <UiTextField v-model="from" type="date" label="From" data-testid="save-template-from" />
        <UiTextField
          v-model="to"
          type="date"
          label="To"
          :error="rangeOk ? undefined : 'At most 366 days, and not before the start'"
          data-testid="save-template-to"
        />
      </div>
      <fieldset class="save__slots">
        <legend>Slots to include</legend>
        <UiCheckbox
          v-for="s in settings.slots.filter((x) => x.id)"
          :key="s.id!"
          :model-value="chosen.includes(s.id!)"
          :label="s.name"
          :data-testid="`save-template-slot-${s.name.toLowerCase()}`"
          @update:model-value="toggle(s.id!, $event)"
        />
      </fieldset>
      <p v-if="error" class="save__error" role="alert">{{ error }}</p>
    </form>
    <template #actions>
      <UiButton variant="text" @click="emit('update:modelValue', false)">Cancel</UiButton>
      <UiButton
        :disabled="!valid"
        :loading="saving"
        data-testid="save-template-confirm"
        @click="submit"
      >
        Save
      </UiButton>
    </template>
  </UiSheet>
</template>

<style scoped>
.save {
  display: grid;
  gap: var(--ui-space-3);
}
.save__dates {
  display: grid;
  gap: var(--ui-space-3);
}
.save__slots {
  display: grid;
  margin: 0;
  padding: 0;
  border: 0;
}
.save__slots legend {
  padding: 0 0 var(--ui-space-1);
  font: var(--ui-type-label-lg);
}
.save__error {
  margin: 0;
  color: var(--ui-error);
}
@media (min-width: 600px) {
  .save__dates {
    grid-template-columns: 1fr 1fr;
  }
}
</style>
