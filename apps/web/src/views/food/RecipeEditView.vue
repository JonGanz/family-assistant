<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { food, type RecipeInput, type RecipeView } from '../../api/food'
import FoodTabs from '../../components/food/FoodTabs.vue'
import LineMatch from '../../components/food/recipes/LineMatch.vue'
import { resizeToJpeg } from '../../lib/imageResize'
import { useRecipesStore } from '../../stores/recipes'
import {
  UiButton,
  UiErrorState,
  UiIconButton,
  UiPage,
  UiSaveBar,
  UiSection,
  UiTextField,
  UiTopAppBar,
  useFormDraft,
} from '../../ui'

interface Form {
  title: string
  servings: string
  prep: string
  cook: string
  total: string
  tags: string
  note: string
  lines: string[]
  steps: string[]
  imageUrl: string | null
  removeImage: boolean
}

const route = useRoute()
const router = useRouter()
const store = useRecipesStore()
const createdId = ref<string | null>(null)
const id = computed(() => (route.params.id ? String(route.params.id) : createdId.value))
const loading = ref(!!id.value)
const loadError = ref<string | null>(null)
const upload = ref<File | null>(null)
const imageError = ref<string | null>(null)
const meta = ref<Pick<RecipeInput, 'import_method' | 'source_url' | 'yield_text'>>({
  import_method: 'manual',
  source_url: null,
  yield_text: null,
})
const source = ref<Form>(blank())

function blank(): Form {
  return {
    title: '',
    servings: '',
    prep: '',
    cook: '',
    total: '',
    tags: '',
    note: '',
    lines: [''],
    steps: [''],
    imageUrl: null,
    removeImage: false,
  }
}
const n2s = (n?: number | null) => (n == null ? '' : String(n))
const s2n = (s: string) => (s.trim() && Number.isFinite(Number(s)) ? Math.round(Number(s)) : null)

function fromView(v: RecipeView): Form {
  return {
    title: v.title,
    servings: n2s(v.servings),
    prep: n2s(v.prep_minutes),
    cook: n2s(v.cook_minutes),
    total: n2s(v.total_minutes),
    tags: v.tags.join(', '),
    note: v.note ?? '',
    lines: v.lines.map((l) => l.raw_text),
    steps: v.steps.length ? [...v.steps] : [''],
    imageUrl: null,
    removeImage: false,
  }
}

onMounted(async () => {
  if (id.value) {
    await store.open(id.value)
    if (store.current) {
      source.value = fromView(store.current)
      meta.value = {
        import_method: store.current.import_method,
        source_url: store.current.source_url ?? null,
        yield_text: store.current.yield_text ?? null,
      }
    } else loadError.value = store.error ?? 'Could not load the recipe'
    loading.value = false
  } else if (store.pendingDraft) {
    const d = store.pendingDraft
    source.value = {
      ...blank(),
      title: d.name,
      servings: n2s(d.servings),
      prep: n2s(d.prep_minutes),
      cook: n2s(d.cook_minutes),
      total: n2s(d.total_minutes),
      tags: d.tags.join(', '),
      lines: d.ingredients.length ? d.ingredients.map((i) => i.text) : [''],
      steps: d.steps.length ? [...d.steps] : [''],
      imageUrl: d.image_url ?? null,
    }
    meta.value = {
      import_method: d.origin,
      source_url: d.source_url ?? null,
      yield_text: d.yield_text ?? null,
    }
  }
})

async function persist(f: Form): Promise<void> {
  const input: RecipeInput = {
    title: f.title.trim(),
    servings: s2n(f.servings),
    prep_minutes: s2n(f.prep),
    cook_minutes: s2n(f.cook),
    total_minutes: s2n(f.total),
    tags: f.tags
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean),
    note: f.note.trim() || null,
    lines: f.lines.map((l) => l.trim()).filter(Boolean),
    steps: f.steps.map((s) => s.trim()).filter(Boolean),
    image_url: id.value ? null : f.imageUrl,
    ...meta.value,
  }
  const saved = await store.save(id.value, input)
  if (!saved) throw new Error('This recipe changed elsewhere. Reload to see the latest.')
  if (f.removeImage && saved.has_image) await food.deleteRecipeImage(saved.id)
  if (upload.value) {
    try {
      await food.uploadRecipeImage(saved.id, upload.value)
    } catch (e) {
      // The recipe is stored; later saves must update it, not create a duplicate.
      createdId.value = saved.id
      throw new Error(
        `The recipe was saved, but the image was not uploaded: ${e instanceof Error ? e.message : 'unknown error'}`,
        { cause: e },
      )
    }
  }
  store.pendingDraft = null
  savedId.value = saved.id
}
const form = useFormDraft<Form>(() => source.value, persist)
const savedId = ref<string | null>(null)
const draft = form.draft

async function save() {
  if (!form.dirty.value && (upload.value || (!id.value && store.pendingDraft))) {
    // An imported draft or a lone image choice leaves the form "clean" but still unsaved (D-109).
    try {
      await persist(JSON.parse(JSON.stringify(draft.value)) as Form)
    } catch (e) {
      form.error.value = e instanceof Error ? e.message : 'Could not save'
      return
    }
  } else if (!(await form.save())) return
  await router.replace(`/food/recipes/${savedId.value}`)
}
function discard() {
  upload.value = null
  form.discard()
}

function move(list: string[], i: number, d: number) {
  const j = i + d
  if (j < 0 || j >= list.length) return
  ;[list[i], list[j]] = [list[j]!, list[i]!]
}
async function onFile(e: Event) {
  const f = (e.target as HTMLInputElement).files?.[0]
  imageError.value = null
  if (!f) return
  try {
    upload.value = await resizeToJpeg(f)
    draft.value.removeImage = false
  } catch (err) {
    upload.value = null
    imageError.value = err instanceof Error ? err.message : 'The image could not be read.'
  }
}
/** The view line behind row `i`, when its text is unchanged (so matching still applies). */
function viewLine(i: number) {
  const l = store.current?.lines.find((x) => x.ord === i)
  return l && l.raw_text === draft.value.lines[i] && l.match_state !== 'matched' ? l : null
}
</script>

<template>
  <UiPage>
    <template #top
      ><UiTopAppBar
        :title="id ? 'Edit recipe' : 'New recipe'"
        back
        @back="router.push('/food/recipes')"
    /></template>
    <FoodTabs />
    <UiErrorState v-if="loadError" :message="loadError" @retry="$router.go(0)" />
    <form v-else-if="!loading" class="form" @submit.prevent="save">
      <UiTextField v-model="draft.title" label="Title" required data-testid="recipe-title" />
      <div class="nums">
        <UiTextField v-model="draft.servings" label="Servings" type="number" />
        <UiTextField v-model="draft.prep" label="Prep (min)" type="number" />
        <UiTextField v-model="draft.cook" label="Cook (min)" type="number" />
        <UiTextField v-model="draft.total" label="Total (min)" type="number" />
      </div>
      <UiTextField v-model="draft.tags" label="Tags (comma separated)" />

      <UiSection heading="Image">
        <p v-if="draft.imageUrl && !draft.removeImage">Using the image from the link.</p>
        <input type="file" accept="image/*" aria-label="Upload an image" @change="onFile" />
        <p v-if="imageError" role="alert" class="image-error" data-testid="image-error">
          {{ imageError }}
        </p>
        <UiButton
          v-if="(draft.imageUrl || id) && !draft.removeImage"
          variant="text"
          @click="((draft.removeImage = true), (upload = null))"
          >Remove image</UiButton
        >
      </UiSection>

      <UiSection heading="Ingredients">
        <div v-for="(_, i) in draft.lines" :key="i" class="row">
          <div class="grow">
            <UiTextField
              v-model="draft.lines[i]!"
              label="Ingredient line"
              :data-testid="`recipe-line-${i}`"
            />
            <LineMatch v-if="id && viewLine(i)" :recipe-id="id" :line="viewLine(i)!" />
          </div>
          <UiIconButton
            icon="arrow_upward"
            label="Move line up"
            @click="move(draft.lines, i, -1)"
          />
          <UiIconButton
            icon="arrow_downward"
            label="Move line down"
            @click="move(draft.lines, i, 1)"
          />
          <UiIconButton icon="delete" label="Remove line" @click="draft.lines.splice(i, 1)" />
        </div>
        <UiButton variant="tonal" icon="add" @click="draft.lines.push('')">Add line</UiButton>
      </UiSection>

      <UiSection heading="Steps">
        <div v-for="(_, i) in draft.steps" :key="i" class="row">
          <div class="grow">
            <UiTextField v-model="draft.steps[i]!" :label="`Step ${i + 1}`" multiline />
          </div>
          <UiIconButton
            icon="arrow_upward"
            label="Move step up"
            @click="move(draft.steps, i, -1)"
          />
          <UiIconButton
            icon="arrow_downward"
            label="Move step down"
            @click="move(draft.steps, i, 1)"
          />
          <UiIconButton icon="delete" label="Remove step" @click="draft.steps.splice(i, 1)" />
        </div>
        <UiButton variant="tonal" icon="add" @click="draft.steps.push('')">Add step</UiButton>
      </UiSection>

      <UiTextField v-model="draft.note" label="Household note" multiline />
    </form>
    <UiSaveBar
      :dirty="form.dirty.value || !!upload || (!id && !!store.pendingDraft)"
      :saving="form.saving.value"
      :error="form.error.value"
      data-testid="recipe-save"
      @save="save"
      @discard="discard"
    />
  </UiPage>
</template>

<style scoped>
.image-error {
  color: var(--ui-error, #b3261e);
}
.form {
  display: grid;
  gap: var(--ui-space-4);
}
.nums {
  display: grid;
  gap: var(--ui-space-3);
  grid-template-columns: repeat(2, 1fr);
}
@media (min-width: 600px) {
  .nums {
    grid-template-columns: repeat(4, 1fr);
  }
}
.row {
  display: flex;
  gap: var(--ui-space-1);
  align-items: flex-start;
}
.grow {
  flex: 1;
  display: grid;
  gap: var(--ui-space-2);
}
</style>
