<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import Modal from './Modal.vue'
import HuePicker from '../components/HuePicker.vue'
import { hueVar, type Hue } from '../lib/hues'
import { S } from './strings'
import {
  distanceTo,
  loadStrokes,
  pathOf,
  saveStrokes,
  type Ink,
  type Shape,
  type Stroke,
} from './whiteboardModel'

defineProps<{ portrait: boolean }>()

const strokes = ref<Stroke[]>(loadStrokes())
const tool = ref<'pen' | 'shape' | 'eraser'>('pen')
const ink = ref<Ink>('ink')
const width = ref(5)
const shape = ref<Shape>('rect')
const WIDTHS = [
  { w: 2, label: S.wbThin },
  { w: 5, label: S.wbMedium },
  { w: 10, label: S.wbThick },
]
const SHAPES: { id: Shape; label: string; d: string }[] = [
  { id: 'rect', label: S.wbRect, d: 'M4 6h16v12H4z' },
  { id: 'circle', label: S.wbCircle, d: 'M4 12a8 8 0 1 0 16 0a8 8 0 1 0 -16 0' },
  { id: 'triangle', label: S.wbTriangle, d: 'M12 4l9 16H3z' },
  { id: 'hexagon', label: S.wbHexagon, d: 'M7 4h10l5 8-5 8H7l-5-8z' },
]
const shapeIcon = computed(() => SHAPES.find((x) => x.id === shape.value)!.d)
const confirming = ref(false)
const board = ref<SVGSVGElement | null>(null)
const live = ref<Stroke | null>(null)
let erasing = false

/** The open option popup, placed beside its tool button (fixed, so the scrolling toolbar cannot clip it). */
const menu = ref<'size' | 'shape' | null>(null)
const menuStyle = ref<Record<string, string>>({})
function openMenu(which: 'size' | 'shape', e: MouseEvent, portrait: boolean) {
  const again = menu.value === which
  if (which === 'shape') tool.value = 'shape'
  if (again) {
    menu.value = null
    return
  }
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
  menuStyle.value = portrait
    ? { left: `${r.left}px`, bottom: `${window.innerHeight - r.top + 8}px` }
    : { top: `${r.top}px`, right: `${window.innerWidth - r.left + 8}px` }
  menu.value = which
}
function pickWidth(w: number) {
  width.value = w
  menu.value = null
}
function pickShape(sh: Shape) {
  shape.value = sh
  menu.value = null
}
function pickTool(t: 'pen' | 'eraser') {
  tool.value = t
  menu.value = null
}

/** An `ink` stroke is black on a light theme and white on a dark one; hues follow the theme. */
const stroke = (c: Ink) => (c === 'ink' ? 'var(--wb-ink)' : hueVar(c))
const picked = computed(() => (ink.value === 'ink' ? ('' as Hue) : ink.value))
/** Choosing a colour with the eraser in hand goes back to the pen; a shape tool stays. */
function pickInk(c: Ink) {
  ink.value = c
  if (tool.value === 'eraser') tool.value = 'pen'
}

function at(e: PointerEvent): [number, number] {
  const r = board.value!.getBoundingClientRect()
  return [Math.round(e.clientX - r.left), Math.round(e.clientY - r.top)]
}
function eraseAt(pt: [number, number]) {
  const keep = strokes.value.filter((s) => distanceTo(s, pt) >= 14 + s.w / 2)
  if (keep.length !== strokes.value.length) strokes.value = keep
}
function down(e: PointerEvent) {
  board.value?.setPointerCapture(e.pointerId)
  menu.value = null
  const pt = at(e)
  if (tool.value === 'eraser') {
    erasing = true
    eraseAt(pt)
  } else if (tool.value === 'shape') {
    live.value = { c: ink.value, w: width.value, s: shape.value, p: [pt, pt] }
  } else {
    live.value = { c: ink.value, w: width.value, p: [pt] }
  }
}
function move(e: PointerEvent) {
  if (erasing) eraseAt(at(e))
  else if (live.value?.s) live.value.p[1] = at(e)
  else if (live.value) live.value.p.push(at(e))
}
/** A shape needs some size: a tap without a drag draws nothing. */
const MIN_SHAPE_PX = 6
function up() {
  const s = live.value
  if (s && (!s.s || Math.hypot(s.p[1]![0] - s.p[0]![0], s.p[1]![1] - s.p[0]![1]) >= MIN_SHAPE_PX)) {
    strokes.value = [...strokes.value, s]
  }
  if (s || erasing) saveStrokes(strokes.value)
  live.value = null
  erasing = false
}
function wipe() {
  strokes.value = []
  saveStrokes([])
  confirming.value = false
}
onBeforeUnmount(up)
</script>

<template>
  <div class="wb" :class="portrait ? 'portrait' : 'landscape'" data-testid="whiteboard">
    <svg
      ref="board"
      class="board"
      :aria-label="S.wbBoard"
      role="img"
      @pointerdown.prevent="down"
      @pointermove="move"
      @pointerup="up"
      @pointercancel="up"
    >
      <path
        v-for="(s, i) in strokes.concat(live ? [live] : [])"
        :key="i"
        :d="pathOf(s)"
        :stroke="stroke(s.c)"
        :stroke-width="s.w"
        fill="none"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
    <div class="tools" role="toolbar" :aria-label="S.wbTools">
      <div class="row">
        <button
          type="button"
          class="tool"
          :class="{ on: tool === 'pen' }"
          :aria-label="S.wbPen"
          :title="S.wbPen"
          @click="pickTool('pen')"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M3 21l3.5-1 12-12-2.5-2.5-12 12L3 21zM15 5l2.5 2.5" />
          </svg>
        </button>
        <button
          type="button"
          class="tool"
          :class="{ on: tool === 'shape' }"
          :aria-label="S.wbShape"
          :aria-expanded="menu === 'shape'"
          :title="S.wbShape"
          @click="openMenu('shape', $event, portrait)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path :d="shapeIcon" /></svg>
        </button>
        <button
          type="button"
          class="tool"
          :class="{ on: tool === 'eraser' }"
          :aria-label="S.wbEraser"
          :title="S.wbEraser"
          @click="pickTool('eraser')"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 16l9-9 6 6-6 6H8l-4-3zM9 11l6 6M12 19h8" />
          </svg>
        </button>
        <button
          type="button"
          class="tool"
          :aria-label="S.wbPenSize"
          :aria-expanded="menu === 'size'"
          :title="S.wbPenSize"
          @click="openMenu('size', $event, portrait)"
        >
          <span class="nib" :style="{ height: `${width}px` }" />
        </button>
      </div>
      <div class="row">
        <button
          type="button"
          class="ink"
          :class="{ on: ink === 'ink' }"
          :aria-label="S.wbInk"
          :title="S.wbInk"
          @click="pickInk('ink')"
        />
        <HuePicker
          class="hues"
          :model-value="picked"
          name="whiteboard-hue"
          hide-legend
          :legend="S.wbColour"
          @update:model-value="pickInk"
        />
      </div>
      <div class="row del">
        <button
          type="button"
          class="tool"
          :aria-label="S.wbClear"
          :title="S.wbClear"
          @click="confirming = true"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6" />
          </svg>
        </button>
      </div>
    </div>
    <template v-if="menu">
      <div class="scrim" @pointerdown="menu = null" />
      <div
        class="flyout"
        role="group"
        :aria-label="menu === 'size' ? S.wbPenSize : S.wbShape"
        :style="menuStyle"
      >
        <template v-if="menu === 'size'">
          <button
            v-for="o in WIDTHS"
            :key="o.w"
            type="button"
            class="tool"
            :class="{ on: width === o.w }"
            :aria-label="o.label"
            :aria-pressed="width === o.w"
            :title="o.label"
            @click="pickWidth(o.w)"
          >
            <span class="nib" :style="{ height: `${o.w}px` }" />
          </button>
        </template>
        <template v-else>
          <button
            v-for="o in SHAPES"
            :key="o.id"
            type="button"
            class="tool"
            :class="{ on: shape === o.id }"
            :aria-label="o.label"
            :aria-pressed="shape === o.id"
            :title="o.label"
            @click="pickShape(o.id)"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true"><path :d="o.d" /></svg>
          </button>
        </template>
      </div>
    </template>
    <Modal v-if="confirming" :title="S.wbClearAsk" @close="confirming = false">
      <div class="confirm">
        <button type="button" class="tool wide" @click="wipe">{{ S.wbYes }}</button>
        <button type="button" class="tool wide" @click="confirming = false">{{ S.wbNo }}</button>
      </div>
    </Modal>
  </div>
</template>

<style>
/* Unscoped: the mode is set on an ancestor (the canvas). */
[data-theme='dark'] .wb.wb {
  --wb-ink: #fff;
}
</style>

<style scoped>
.wb {
  --wb-ink: #000;
  --swatch-size: 3rem;
  --swatch-glow: var(--wb-ink);
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  gap: var(--space-2);
  padding: var(--space-2);
  box-sizing: border-box;
}
.wb.portrait {
  flex-direction: column;
}
.board {
  flex: 1;
  min-width: 0;
  min-height: 0;
  width: 100%;
  height: 100%;
  border: 2px solid var(--color-border);
  background: var(--color-bg);
  touch-action: none;
  cursor: crosshair;
}
.tools {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  width: 9.5rem;
  /* Room for the selected swatch's glow, which the scroll container would clip. */
  padding: 0.9rem;
  box-sizing: border-box;
  overflow-y: auto;
}
.portrait .tools {
  flex-direction: row;
  flex-wrap: wrap;
  width: auto;
  align-items: center;
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  align-items: center;
}
.del {
  margin-top: auto;
}
.portrait .del {
  margin: 0 0 0 auto;
}
.tool {
  display: grid;
  place-items: center;
  min-width: var(--min-touch);
  min-height: var(--min-touch);
  border: 2px solid var(--color-border);
  background: var(--color-bg);
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.tool.on {
  background: var(--color-accent);
  border-color: var(--color-accent);
  color: var(--color-on-accent);
}
.tool svg {
  width: 1.6rem;
  height: 1.6rem;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.nib {
  width: 1.4rem;
  background: currentColor;
  border-radius: 99px;
}
.ink {
  flex: none;
  width: 3rem;
  height: 3rem;
  aspect-ratio: 1;
  padding: 0;
  border: 2px solid var(--color-border);
  border-radius: 50%;
  background: var(--wb-ink);
  cursor: pointer;
}
.ink.on {
  border-color: var(--wb-ink);
  box-shadow:
    0 0 0 2px var(--color-bg) inset,
    0 0 10px 3px var(--wb-ink);
}
.scrim {
  position: fixed;
  inset: 0;
  z-index: 9;
}
.flyout {
  position: fixed;
  z-index: 10;
  display: flex;
  gap: var(--space-2);
  padding: var(--space-2);
  border: 2px solid var(--color-border);
  background: var(--color-surface);
  box-shadow: 0 8px 32px rgb(0 0 0 / 35%);
}
.confirm {
  display: flex;
  gap: var(--space-3);
}
.wide {
  flex: 1;
  padding-inline: var(--space-4);
}
</style>
