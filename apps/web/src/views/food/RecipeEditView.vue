<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { food, type RecipeInput, type RecipeView } from '../../api/food'
import FoodTabs from '../../components/food/FoodTabs.vue'
import LineMatch from '../../components/food/recipes/LineMatch.vue'
import SectionItems from '../../components/food/recipes/SectionItems.vue'
import {
  addSection,
  blankSections,
  deleteSection,
  fromDraft,
  fromView as sectionsFromView,
  mergeSection,
  moveItem,
  moveSection,
  toInput,
  type Place,
  type SectionForm,
} from '../../components/food/recipes/sectionsForm'
import { resizeToJpeg } from '../../lib/imageResize'
import { useRecipesStore } from '../../stores/recipes'
import { applyReimport } from '../../stores/recipesReimport'
import {
  UiButton,
  UiDialog,
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
  sections: SectionForm[]
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
const confirmReimport = ref(false)
const reimporting = ref(false)
const reimported = ref(false)
const reimportError = ref<string | null>(null)

function blank(): Form {
  return {
    title: '',
    servings: '',
    prep: '',
    cook: '',
    total: '',
    tags: '',
    note: '',
    ...blankSections(),
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
    ...sectionsFromView(v),
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
    if (route.query.reimport === '1') {
      // Handled once: a reload must not import again.
      await router.replace({ query: { ...route.query, reimport: undefined } })
      await reimport()
    }
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
      ...fromDraft(d),
      imageUrl: d.image_url ?? null,
    }
    meta.value = {
      import_method: d.origin,
      source_url: d.source_url ?? null,
      yield_text: d.yield_text ?? null,
    }
  }
})

/** Replaces the imported parts of the form with the source page's current version (D-126). */
async function reimport() {
  confirmReimport.value = false
  const url = store.current?.source_url
  if (!url || reimporting.value) return
  reimporting.value = true
  reimportError.value = null
  try {
    const d = await food.importRecipe({ url })
    meta.value = { ...meta.value, ...applyReimport(draft.value, d) }
    reimported.value = true
  } catch (e) {
    reimportError.value = e instanceof Error ? e.message : 'The page could not be imported.'
  } finally {
    reimporting.value = false
  }
}

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
    ...toInput(f),
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
  reimported.value = false
  if (store.current)
    meta.value = {
      import_method: store.current.import_method,
      source_url: store.current.source_url ?? null,
      yield_text: store.current.yield_text ?? null,
    }
  form.discard()
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
/** The view line with this text, while it is unmatched or proposed (so matching still applies). */
function viewLine(text: string) {
  const l = store.current?.lines.find((x) => x.raw_text === text)
  return l && l.match_state !== 'matched' ? l : null
}
const targets = computed<{ value: Place; label: string }[]>(() => [
  { value: 'top', label: 'Shared / finish' },
  ...draft.value.sections.map((s, i) => ({
    value: i as Place,
    label: s.name.trim() || `Section ${i + 1}`,
  })),
])
const mergeTargets = (i: number) =>
  draft.value.sections
    .map((s, k) => ({ k, label: s.name.trim() || `Section ${k + 1}` }))
    .filter((x) => x.k !== i)
function onMerge(i: number, e: Event) {
  const el = e.target as HTMLSelectElement
  if (el.value !== '') mergeSection(draft.value, i, Number(el.value))
  el.value = ''
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
      <div v-if="id && meta.source_url" class="reimport">
        <UiButton
          variant="tonal"
          icon="refresh"
          :disabled="reimporting"
          data-testid="recipe-reimport"
          @click="confirmReimport = true"
          >{{ reimporting ? 'Importing…' : 'Re-import from source' }}</UiButton
        >
        <p v-if="reimported" role="status" data-testid="recipe-reimport-notice">
          The form holds the re-imported version. Press Save to keep it or Discard to go back.
        </p>
        <p
          v-if="reimportError"
          role="alert"
          class="image-error"
          data-testid="recipe-reimport-error"
        >
          {{ reimportError }}
        </p>
      </div>
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
        <SectionItems
          v-model:items="draft.lines"
          kind="line"
          place="top"
          :targets="targets"
          test-prefix="recipe-line"
          @move="(i, to) => moveItem(draft, 'lines', 'top', i, to)"
        >
          <template #extra="{ index }">
            <LineMatch
              v-if="id && viewLine(draft.lines[index]!)"
              :recipe-id="id"
              :line="viewLine(draft.lines[index]!)!"
            />
          </template>
        </SectionItems>
      </UiSection>

      <UiSection
        v-for="(sec, si) in draft.sections"
        :key="si"
        :heading="`Section ${si + 1}`"
        :data-testid="`recipe-section-edit-${si}`"
      >
        <div class="row">
          <div class="grow">
            <UiTextField
              v-model="sec.name"
              label="Section name"
              :data-testid="`recipe-section-name-${si}`"
            />
          </div>
          <UiIconButton
            icon="arrow_upward"
            label="Move section up"
            @click="moveSection(draft, si, -1)"
          />
          <UiIconButton
            icon="arrow_downward"
            label="Move section down"
            @click="moveSection(draft, si, 1)"
          />
          <UiIconButton icon="delete" label="Delete section" @click="deleteSection(draft, si)" />
        </div>
        <select
          v-if="draft.sections.length > 1"
          class="moveto"
          aria-label="Merge this section into"
          @change="onMerge(si, $event)"
        >
          <option value="" selected disabled>Merge into…</option>
          <option v-for="t in mergeTargets(si)" :key="t.k" :value="String(t.k)">
            {{ t.label }}
          </option>
        </select>
        <h3>Ingredients</h3>
        <SectionItems
          v-model:items="sec.lines"
          kind="line"
          :place="si"
          :targets="targets"
          :test-prefix="`recipe-sec-${si}-line`"
          @move="(i, to) => moveItem(draft, 'lines', si, i, to)"
        />
        <h3>Steps</h3>
        <SectionItems
          v-model:items="sec.steps"
          kind="step"
          :place="si"
          :targets="targets"
          :test-prefix="`recipe-sec-${si}-step`"
          @move="(i, to) => moveItem(draft, 'steps', si, i, to)"
        />
      </UiSection>
      <UiButton
        variant="tonal"
        icon="add"
        data-testid="recipe-add-section"
        @click="addSection(draft)"
        >Add section</UiButton
      >

      <UiSection :heading="draft.sections.length ? 'Finish steps' : 'Steps'">
        <SectionItems
          v-model:items="draft.steps"
          kind="step"
          place="top"
          :targets="targets"
          test-prefix="recipe-step"
          @move="(i, to) => moveItem(draft, 'steps', 'top', i, to)"
        />
      </UiSection>

      <UiTextField v-model="draft.note" label="Household note" multiline />
    </form>
    <UiDialog v-model="confirmReimport" title="Re-import from source?">
      <p>
        Ingredients, steps and sections will be replaced by the page's current version. Nothing is
        saved until you press Save.
      </p>
      <template #actions>
        <UiButton variant="text" @click="confirmReimport = false">Cancel</UiButton>
        <UiButton data-testid="recipe-reimport-confirm" @click="reimport">Re-import</UiButton>
      </template>
    </UiDialog>
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
.reimport {
  display: grid;
  gap: var(--ui-space-2);
  justify-items: start;
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
.moveto {
  min-height: var(--ui-touch);
  font: inherit;
  color: var(--ui-on-surface);
  background: var(--ui-surface-container, transparent);
  border: 1px solid var(--ui-outline-variant, currentColor);
  border-radius: var(--ui-radius-md);
  padding: 0 var(--ui-space-2);
}
</style>
