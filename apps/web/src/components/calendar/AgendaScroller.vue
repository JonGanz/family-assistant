<script setup lang="ts">
// The Agenda: starts at `start`, loads earlier/later date chunks as sentinels scroll into view,
// and keeps at most MAX_CHUNKS chunks (data and DOM), dropping the far end.
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import EventCard from '../EventCard.vue'
import { dayKey, formatDayHeading, groupByDay } from '../../lib/time'
import type { EventLike, MemberLike } from '../../lib/types'
import { toEventLike } from '../../views/shared'
import { useCalendarStore } from '../../stores/calendar'
import {
  chunkAt,
  grow,
  mergeEvents,
  nextChunk,
  spanOf,
  type Chunk,
  type Side,
} from './agendaWindow'

const props = defineProps<{
  tz: string
  /** Day key the agenda starts at. */
  start: string
  /** Bump to rebuild the window at `start` even when `start` did not change. */
  nonce?: number
  members: MemberLike[]
  /** One API call per entry (undefined = everyone). */
  memberKeys: (string | undefined)[]
}>()
defineEmits<{ select: [event: EventLike] }>()

const calendar = useCalendarStore()
const chunks = ref<Chunk[]>([])
const loaded = new Map<string, { chunk: Chunk; tz: string; member?: string }>()
const busy = ref(false)
const stalled = ref<Record<Side, boolean>>({ before: false, after: false })
const root = ref<HTMLElement | null>(null)
const topEl = ref<HTMLElement | null>(null)
const bottomEl = ref<HTMLElement | null>(null)
let observer: IntersectionObserver | null = null
let generation = 0
const MAX_CHAIN = 4

const lkey = (c: Chunk, tz: string, m?: string) => `${c.from}|${c.to}|${tz}|${m ?? ''}`

async function fetchChunk(c: Chunk) {
  await Promise.all(
    props.memberKeys.map((m) => {
      loaded.set(lkey(c, props.tz, m), { chunk: c, tz: props.tz, member: m })
      return calendar.ensureRange(c.from, c.to, props.tz, m)
    }),
  )
}
function dropChunk(c: Chunk) {
  for (const [k, v] of [...loaded]) {
    if (v.chunk.from !== c.from) continue
    calendar.release(c.from, c.to, v.tz, v.member)
    loaded.delete(k)
  }
}
function dropAll() {
  for (const v of loaded.values()) calendar.release(v.chunk.from, v.chunk.to, v.tz, v.member)
  loaded.clear()
}

const events = computed(() =>
  mergeEvents(
    chunks.value.flatMap((c) =>
      props.memberKeys.map((m) => calendar.eventsIn(c.from, c.to, props.tz, m)),
    ),
  ),
)
const today = computed(() => dayKey(new Date(), props.tz))
const groups = computed(() => {
  if (!chunks.value.length) return []
  const { from, to } = spanOf(chunks.value)
  const days = Math.round((Date.parse(`${to}T00:00:00Z`) - Date.parse(`${from}T00:00:00Z`)) / 864e5)
  const likes = events.value.map((e) => toEventLike(e, props.members))
  return [...groupByDay(likes, props.tz, from, days)]
    .filter(([k, list]) => list.length > 0 || k === today.value)
    .map(([key, list]) => ({ key, heading: formatDayHeading(key, props.tz, today.value), list }))
})

/** The first day section under the top of the viewport, and where it sits. */
function anchorOf(): { day: string; top: number } | null {
  for (const s of root.value?.querySelectorAll<HTMLElement>('[data-day]') ?? []) {
    const r = s.getBoundingClientRect()
    if (r.bottom > 0) return { day: s.dataset.day!, top: r.top }
  }
  return null
}
/** Keeps the anchor day where it was after content above it changed. */
function restore(a: { day: string; top: number } | null) {
  if (!a) return
  const s = root.value?.querySelector<HTMLElement>(`[data-day="${a.day}"]`)
  if (!s) return
  const d = s.getBoundingClientRect().top - a.top
  if (d) window.scrollBy(0, d)
}
const near = (el: HTMLElement | null) => {
  if (!el) return false
  const r = el.getBoundingClientRect()
  return r.bottom > -400 && r.top < window.innerHeight + 400
}

async function load(side: Side, chain = 0): Promise<void> {
  if (busy.value || !chunks.value.length) return
  const gen = generation
  busy.value = true
  stalled.value[side] = false
  let produced: boolean
  try {
    const next = nextChunk(chunks.value, side)
    await fetchChunk(next)
    if (gen !== generation) return
    const a = anchorOf()
    const r = grow(chunks.value, side)
    chunks.value = r.chunks
    for (const c of r.evicted) dropChunk(c)
    await nextTick()
    restore(a)
    produced = events.value.some((e) => e.start_at >= next.from && e.start_at < next.to)
  } finally {
    busy.value = false
  }
  if (gen !== generation) return
  const el = side === 'before' ? topEl.value : bottomEl.value
  if (chain + 1 < (produced ? MAX_CHAIN + 2 : MAX_CHAIN) && near(el)) await load(side, chain + 1)
  else if (near(el)) stalled.value[side] = true
}

async function reset() {
  generation++
  busy.value = false
  dropAll()
  stalled.value = { before: false, after: false }
  const first = chunkAt(props.start)
  chunks.value = [first]
  await fetchChunk(first)
  window.scrollTo({ top: 0 })
}

onMounted(() => {
  void reset()
  if (typeof IntersectionObserver === 'undefined') return
  observer = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue
        void load(e.target === topEl.value ? 'before' : 'after')
      }
    },
    { rootMargin: '400px 0px' },
  )
  if (topEl.value) observer.observe(topEl.value)
  if (bottomEl.value) observer.observe(bottomEl.value)
})
onBeforeUnmount(() => {
  observer?.disconnect()
  observer = null
  generation++
  dropAll()
})
watch(
  () => [props.start, props.nonce, props.tz, props.memberKeys.join('|')],
  () => void reset(),
)
</script>

<template>
  <div ref="root" class="agenda">
    <div ref="topEl" class="edge" data-edge="before" aria-live="polite">
      <button v-if="stalled.before" type="button" class="more" @click="load('before')">
        Load earlier
      </button>
      <span v-else-if="busy" class="note">Loading…</span>
    </div>
    <section v-for="g in groups" :key="g.key" class="day" :data-day="g.key">
      <h2 class="head" :class="{ now: g.key === today }">{{ g.heading }}</h2>
      <p v-if="!g.list.length" class="empty">Nothing scheduled</p>
      <div v-else class="list">
        <EventCard
          v-for="e in g.list"
          :key="e.id"
          :event="e"
          :tz="tz"
          :members="members"
          selectable
          @select="$emit('select', $event)"
        />
      </div>
    </section>
    <div ref="bottomEl" class="edge" data-edge="after" aria-live="polite">
      <button v-if="stalled.after" type="button" class="more" @click="load('after')">
        Load later
      </button>
      <span v-else-if="busy" class="note">Loading…</span>
    </div>
  </div>
</template>

<style scoped>
.agenda {
  display: grid;
  gap: var(--ui-space-4);
  /* The scroll position is kept by hand when days are added above. */
  overflow-anchor: none;
}
.edge {
  min-height: 1px;
  text-align: center;
}
.head {
  position: sticky;
  top: var(--ui-appbar-h, 64px);
  z-index: 1;
  margin: 0 0 var(--ui-space-2);
  padding: var(--ui-space-1) 0;
  background: var(--ui-surface);
  font: var(--ui-type-title-md);
}
.head.now {
  color: var(--ui-primary);
}
.list {
  display: grid;
  gap: var(--ui-space-2);
}
.empty,
.note {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
  margin: 0;
}
.more {
  background: none;
  border: 0;
  cursor: pointer;
  color: var(--ui-primary);
  font: var(--ui-type-label-lg);
}
</style>
