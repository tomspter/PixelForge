<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type Konva from 'konva'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { useTemplateStore } from '../stores/template'
import type { TemplateField } from '../types/template'
import { formatDateTime } from '../utils/dateTime'

const store = useTemplateStore()
const host = ref<HTMLDivElement>()
const stage = ref<InstanceType<any>>()
const transformer = ref<InstanceType<any>>()
const hostSize = ref({ width: 800, height: 600 })
const image = ref<HTMLImageElement>()
const inpaintPatches = ref<Record<string, HTMLImageElement>>({})
const textPatches = ref<Record<string, HTMLImageElement>>({})
const patchUrls = new Map<string, string>()
const textPatchUrls = new Map<string, string>()
const drawing = ref<{ x: number; y: number } | null>(null)
const draft = ref({ x: 0, y: 0, width: 0, height: 0 })
const fittedUrl = ref('')
let observer: ResizeObserver
let patchTimer: ReturnType<typeof setTimeout> | undefined
let patchGeneration = 0

const scale = computed(() => store.zoom)
const stageSize = computed(() => store.document ? ({ width: store.document.background.width * scale.value, height: store.document.background.height * scale.value }) : ({ width: 0, height: 0 }))
const canvasFont = (family: string) => ({ '宋体': 'Songti SC, SimSun', '仿宋': 'STFangsong, FangSong', '黑体': 'Heiti SC, SimHei, PingFang SC', 'Arial': 'Arial', 'Times New Roman': 'Times New Roman' }[family] ?? family)
const previewText = (f: TemplateField) => {
  if (f.kind === 'csv') return f.csvColumn ? `{${f.csvColumn}}` : 'CSV'
  if (f.kind === 'random') {
    const min = f.randomMin ?? 0; const max = f.randomMax ?? 100; const decimals = Math.min(8, Math.max(0, f.randomDecimals ?? 0))
    return ((min + max) / 2).toFixed(decimals)
  }
  if (f.kind === 'date') return formatDateTime(`${f.dateValue ?? ''}T${f.timeValue ?? ''}`, f.dateFormat)
  return f.value || f.name
}

watch(() => store.backgroundUrl, url => {
  image.value = undefined
  if (!url) return
  const el = new Image()
  el.onload = () => {
    image.value = el
    if (fittedUrl.value !== url && store.document) {
      const availableWidth = Math.max(1, hostSize.value.width - 96)
      const availableHeight = Math.max(1, hostSize.value.height - 96)
      const fit = Math.min(availableWidth / store.document.background.width, availableHeight / store.document.background.height, 1)
      store.zoom = Math.max(0.05, Math.round(fit * 100) / 100)
      fittedUrl.value = url
    }
  }
  el.onerror = () => { store.notice = '图片加载失败，请确认文件仍然存在且格式有效' }
  el.src = url
}, { immediate: true })
watch([() => store.selectedId, () => store.tool], () => requestAnimationFrame(() => {
  const s = stage.value?.getNode()
  const node = store.tool === 'select' ? s?.findOne(`#field-${store.selectedId}`) : null
  transformer.value?.getNode()?.nodes(node ? [node] : [])
}))
watch(() => store.tool, tool => { if (tool !== 'draw') drawing.value = null })
watch(() => JSON.stringify({ path: store.document?.background.path, fields: store.document?.fields.map(f => ({ id: f.id, enabled: f.enabled, mode: f.clear.mode, eraseRect: f.eraseRect, layoutRect: f.layoutRect, threshold: f.clear.inpaintThreshold, radius: f.clear.inpaintRadius, kind: f.kind, value: f.value, csvColumn: f.csvColumn, dateValue: f.dateValue, timeValue: f.timeValue, dateFormat: f.dateFormat, randomMin: f.randomMin, randomMax: f.randomMax, randomDecimals: f.randomDecimals, text: f.text })) }), () => {
  clearTimeout(patchTimer)
  patchTimer = setTimeout(refreshEditorPatches, 180)
}, { immediate: true })

onMounted(() => { observer = new ResizeObserver(([entry]) => hostSize.value = { width: entry.contentRect.width, height: entry.contentRect.height }); if (host.value) observer.observe(host.value) })
onBeforeUnmount(() => { observer?.disconnect(); clearTimeout(patchTimer); patchUrls.forEach(url => URL.revokeObjectURL(url)); textPatchUrls.forEach(url => URL.revokeObjectURL(url)) })

async function loadPatch(bytes: number[]) {
  const url = URL.createObjectURL(new Blob([Uint8Array.from(bytes).buffer], { type: 'image/png' }))
  const el = new Image()
  await new Promise<void>((resolve, reject) => { el.onload = () => resolve(); el.onerror = reject; el.src = url })
  return { el, url }
}

async function refreshEditorPatches() {
  if (!isTauri() || !store.document) return
  const generation = ++patchGeneration
  const fields = store.document.fields.filter(f => f.enabled && f.clear.mode === 'inpaint')
  const textFields = store.document.fields.filter(f => f.enabled)
  const next: Record<string, HTMLImageElement> = {}
  const nextText: Record<string, HTMLImageElement> = {}
  await Promise.all([...fields.map(async field => {
    try {
      const bytes = await invoke<number[]>('render_inpaint_patch', { backgroundPath: store.document!.background.path, rect: field.eraseRect, threshold: field.clear.inpaintThreshold ?? 14, radius: field.clear.inpaintRadius ?? 2 })
      if (generation !== patchGeneration) return
      const { el, url } = await loadPatch(bytes)
      const old = patchUrls.get(field.id); if (old) URL.revokeObjectURL(old)
      patchUrls.set(field.id, url); next[field.id] = el
    } catch (error) { store.notice = `智能抹除预览失败：${String(error)}` }
  }), ...textFields.map(async field => {
    try {
      const bytes = await invoke<number[]>('render_text_patch', { field, value: previewText(field) })
      if (generation !== patchGeneration) return
      const { el, url } = await loadPatch(bytes)
      const old = textPatchUrls.get(field.id); if (old) URL.revokeObjectURL(old)
      textPatchUrls.set(field.id, url); nextText[field.id] = el
    } catch (error) { store.notice = `文字预览失败：${String(error)}` }
  })])
  if (generation === patchGeneration) {
    const active = new Set(fields.map(f => f.id))
    for (const [id, url] of patchUrls) if (!active.has(id)) { URL.revokeObjectURL(url); patchUrls.delete(id) }
    const activeText = new Set(textFields.map(f => f.id))
    for (const [id, url] of textPatchUrls) if (!activeText.has(id)) { URL.revokeObjectURL(url); textPatchUrls.delete(id) }
    inpaintPatches.value = next
    textPatches.value = nextText
  }
}

function pointer() { const p = stage.value?.getNode().getPointerPosition(); return p ? { x: Math.round(p.x / scale.value), y: Math.round(p.y / scale.value) } : null }
function onDown(e: Konva.KonvaEventObject<MouseEvent>) {
  if (store.tool !== 'draw' || e.target !== e.target.getStage()) return
  const p = pointer(); if (!p) return; drawing.value = p; draft.value = { ...p, width: 0, height: 0 }
}
function onMove() { if (!drawing.value) return; const p = pointer(); if (!p) return; draft.value = { x: Math.min(drawing.value.x, p.x), y: Math.min(drawing.value.y, p.y), width: Math.abs(p.x - drawing.value.x), height: Math.abs(p.y - drawing.value.y) } }
function onUp() { if (drawing.value && draft.value.width >= 8 && draft.value.height >= 8) store.addField(draft.value); drawing.value = null }
function updateRect(id: string, e: Konva.KonvaEventObject<any>, moveErase = false) {
  const f = store.document?.fields.find(x => x.id === id); if (!f) return
  const previous = { ...f.layoutRect }
  const n = e.target; const rect = { x: Math.max(0, Math.round(n.x())), y: Math.max(0, Math.round(n.y())), width: Math.round(n.width() * n.scaleX()), height: Math.round(n.height() * n.scaleY()) }
  if (moveErase) {
    f.eraseRect.x = Math.max(0, f.eraseRect.x + rect.x - previous.x)
    f.eraseRect.y = Math.max(0, f.eraseRect.y + rect.y - previous.y)
  }
  n.x(rect.x); n.y(rect.y)
  n.scaleX(1); n.scaleY(1); f.layoutRect = rect; store.markDirty()
}
</script>

<template>
  <div ref="host" class="checkerboard relative h-full w-full overflow-auto scroll-thin" :class="store.tool === 'draw' ? 'cursor-draw' : 'cursor-select'">
    <div v-if="!store.document" class="absolute inset-0 grid place-items-center">
      <button class="group rounded-xl border border-dashed border-base-content/20 bg-base-100/80 px-16 py-14 text-center transition hover:border-primary/60 hover:bg-base-200" @click="store.importBackground">
        <span class="mb-3 block text-4xl font-light text-primary/70 transition group-hover:scale-110">＋</span>
        <span class="block text-sm font-semibold">导入 PNG / JPG，开始铸造模板</span>
        <!-- <span class="mt-2 block text-xs text-base-content/40">原始像素坐标 · 无损画布</span> -->
      </button>
    </div>
    <div v-else class="m-12 inline-block shadow-2xl shadow-black/50" :style="{ width: `${stageSize.width}px`, height: `${stageSize.height}px` }">
      <v-stage ref="stage" :config="{ ...stageSize, scaleX: scale, scaleY: scale }" @mousedown="onDown" @mousemove="onMove" @mouseup="onUp">
        <v-layer><v-image :config="{ image, width: store.document.background.width, height: store.document.background.height, listening: false }" /></v-layer>
        <v-layer>
          <template v-for="f in store.document.fields.filter(x => x.enabled)" :key="`mask-${f.id}`">
            <v-image v-if="f.clear.mode === 'inpaint' && inpaintPatches[f.id]" :config="{ image: inpaintPatches[f.id], ...f.eraseRect, listening: false }" />
          </template>
        </v-layer>
        <v-layer>
          <template v-for="f in store.document.fields.filter(x => x.enabled)" :key="`text-${f.id}`">
            <v-image v-if="textPatches[f.id]" :config="{ image: textPatches[f.id], ...f.layoutRect, listening: false }" />
            <v-text v-else :config="{ x: f.layoutRect.x, y: f.layoutRect.y, width: f.layoutRect.width, height: f.layoutRect.height, padding: f.text.padding, text: previewText(f), fontFamily: canvasFont(f.text.fontFamily), fontSize: f.text.fontSize, fill: f.text.color, align: f.text.horizontalAlign, verticalAlign: f.text.verticalAlign, lineHeight: f.text.lineHeight, letterSpacing: f.text.letterSpacing, wrap: 'none', listening: false }" />
          </template>
        </v-layer>
        <v-layer>
          <v-rect v-for="f in store.document.fields" :key="f.id" :config="{ id: `field-${f.id}`, ...f.layoutRect, stroke: f.id === store.selectedId ? '#0d99ff' : 'rgba(13,153,255,.72)', strokeWidth: 1 / scale, dash: f.id === store.selectedId ? [] : [5 / scale, 4 / scale], fill: f.id === store.selectedId ? 'rgba(13,153,255,.06)' : 'transparent', draggable: store.tool === 'select', listening: store.tool === 'select' }" @click="store.selectedId = f.id" @dragend="updateRect(f.id, $event, true)" @transformend="updateRect(f.id, $event)" />
          <v-rect v-if="drawing" :config="{ ...draft, stroke: '#0d99ff', strokeWidth: 1 / scale, dash: [5 / scale, 3 / scale], fill: 'rgba(13,153,255,.08)', listening: false }" />
          <v-transformer ref="transformer" :config="{ rotateEnabled: false, borderStroke: '#0d99ff', borderStrokeWidth: 1 / scale, anchorStroke: '#0d99ff', anchorStrokeWidth: 1.5 / scale, anchorFill: '#ffffff', anchorSize: 9 / scale, anchorCornerRadius: 999, keepRatio: false, enabledAnchors: ['top-left','top-center','top-right','middle-left','middle-right','bottom-left','bottom-center','bottom-right'], boundBoxFunc: (_old: unknown, next: any) => next.width < 8 || next.height < 8 ? _old : next }" />
        </v-layer>
      </v-stage>
    </div>
  </div>
</template>
