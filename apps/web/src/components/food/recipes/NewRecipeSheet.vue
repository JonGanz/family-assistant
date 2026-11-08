<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { problemCode } from '../../../api/http'
import { useRecipesStore } from '../../../stores/recipes'
import { UiButton, UiProgress, UiSheet, UiTextField } from '../../../ui'

const open = defineModel<boolean>({ required: true })
const router = useRouter()
const store = useRecipesStore()
const mode = ref<'choose' | 'url' | 'text'>('choose')
const url = ref('')
const text = ref('')
const busy = ref(false)
const failure = ref<string | null>(null)

const messages: Record<string, string> = {
  no_recipe: 'That page has no recipe we can read.',
  fetch_failed: 'Could not fetch that page.',
  llm_unavailable: 'The reading assistant is not available right now.',
  llm_failed: 'The reading assistant could not make sense of it.',
}

function reset() {
  mode.value = 'choose'
  failure.value = null
  busy.value = false
}

async function typeIt() {
  store.pendingDraft = null
  open.value = false
  reset()
  await router.push({ name: 'food-recipe-new' })
}

async function submit() {
  busy.value = true
  failure.value = null
  try {
    await store.importDraft(mode.value === 'url' ? { url: url.value.trim() } : { text: text.value })
    open.value = false
    reset()
    await router.push({ name: 'food-recipe-new' })
  } catch (e) {
    const code = problemCode(e)
    failure.value =
      (code && messages[code]) ||
      (e instanceof Error && e.message) ||
      'Could not import that recipe.'
    busy.value = false
  }
}
</script>

<template>
  <UiSheet v-model="open" title="New recipe" @update:model-value="(v) => !v && reset()">
    <div class="new">
      <template v-if="mode === 'choose'">
        <UiButton variant="tonal" icon="link" data-testid="recipe-import-url" @click="mode = 'url'"
          >From a link</UiButton
        >
        <UiButton
          variant="tonal"
          icon="edit"
          data-testid="recipe-import-text"
          @click="mode = 'text'"
          >Paste text</UiButton
        >
        <UiButton variant="tonal" icon="add" data-testid="recipe-type-in" @click="typeIt"
          >Type it in</UiButton
        >
      </template>
      <template v-else>
        <UiTextField
          v-if="mode === 'url'"
          v-model="url"
          label="Recipe link"
          type="url"
          data-testid="recipe-import-url-field"
        />
        <UiTextField
          v-else
          v-model="text"
          label="Recipe text"
          multiline
          data-testid="recipe-import-text-field"
        />
        <UiProgress v-if="busy" />
        <p v-if="busy">Reading the recipe. This can take a minute.</p>
        <p v-if="failure" role="alert" class="fail" data-testid="recipe-import-error">
          {{ failure }}
        </p>
        <div class="row">
          <UiButton variant="text" :disabled="busy" @click="reset">Back</UiButton>
          <UiButton v-if="failure" variant="tonal" data-testid="recipe-type-in" @click="typeIt"
            >Type it in</UiButton
          >
          <UiButton
            :loading="busy"
            :disabled="mode === 'url' ? !url.trim() : !text.trim()"
            data-testid="recipe-import-submit"
            @click="submit"
            >Import</UiButton
          >
        </div>
      </template>
    </div>
  </UiSheet>
</template>

<style scoped>
.new {
  display: grid;
  gap: var(--ui-space-3);
}
.row {
  display: flex;
  gap: var(--ui-space-2);
  justify-content: flex-end;
  flex-wrap: wrap;
}
.fail {
  color: var(--ui-error);
}
</style>
