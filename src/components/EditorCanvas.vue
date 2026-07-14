<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type Konva from 'konva'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { ImagePlus } from '@lucide/vue'
import { useFeedbackStore } from '../stores/feedback'
import { useTemplateStore } from '../stores/template'
import type { TemplateField } from '../types/template'
import { formatDateTime } from '../utils/dateTime'

const store = useTemplateStore()
const feedback = useFeedbackStore()
const host = ref<HTMLDivElement>()
const stage = ref<InstanceType<any>>()
const transformer = ref<InstanceType<any>>()
const hostSize = ref({ width: 800, height: 600 })
const image = ref<HTMLImageElement>()
interface PatchPayload { x: number; y: number; width: number; height: number; bytes: number[] }
interface CanvasPatch extends Omit<PatchPayload, 'bytes'> { image: HTMLImageElement }
const clearPatches = ref<Record<string, CanvasPatch>>({})
const textPatches = ref<Record<string, CanvasPatch>>({})
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
  el.onerror = () => { feedback.setNotice('图片加载失败，请确认文件仍然存在且格式有效', 'error') }
  el.src = url
}, { immediate: true })
watch([() => store.selectedId, () => store.tool, () => store.selected?.rotationEnabled], () => requestAnimationFrame(() => {
  const s = stage.value?.getNode()
  const node = store.tool === 'select' ? s?.findOne(`#field-${store.selectedId}`) : null
  const transformerNode = transformer.value?.getNode()
  transformerNode?.nodes(node ? [node] : [])
  transformerNode?.forceUpdate()
}))
watch(() => store.tool, tool => { if (tool !== 'draw') drawing.value = null })
watch(() => JSON.stringify({ path: store.document?.background.path, fields: store.document?.fields.map(f => ({ id: f.id, enabled: f.enabled, mode: f.clear.mode, eraseRect: f.eraseRect, layoutRect: f.layoutRect, rotationEnabled: f.rotationEnabled, rotation: f.rotation, patchPath: f.clear.patchPath, inpaintThreshold: f.clear.inpaintThreshold, inpaintRadius: f.clear.inpaintRadius, teleaThreshold: f.clear.teleaThreshold, teleaMaskRadius: f.clear.teleaMaskRadius, teleaRadius: f.clear.teleaRadius, kind: f.kind, value: f.value, csvColumn: f.csvColumn, dateValue: f.dateValue, timeValue: f.timeValue, dateFormat: f.dateFormat, randomMin: f.randomMin, randomMax: f.randomMax, randomDecimals: f.randomDecimals, text: f.text })) }), () => {
  clearTimeout(patchTimer)
  patchTimer = setTimeout(refreshEditorPatches, 180)
}, { immediate: true })

onMounted(() => { observer = new ResizeObserver(([entry]) => hostSize.value = { width: entry.contentRect.width, height: entry.contentRect.height }); if (host.value) observer.observe(host.value) })
onBeforeUnmount(() => { observer?.disconnect(); clearTimeout(patchTimer); patchUrls.forEach(url => URL.revokeObjectURL(url)); textPatchUrls.forEach(url => URL.revokeObjectURL(url)) })

async function loadPatch(payload: PatchPayload) {
  const url = URL.createObjectURL(new Blob([Uint8Array.from(payload.bytes).buffer], { type: 'image/png' }))
  const el = new Image()
  await new Promise<void>((resolve, reject) => { el.onload = () => resolve(); el.onerror = reject; el.src = url })
  return { patch: { image: el, x: payload.x, y: payload.y, width: payload.width, height: payload.height }, url }
}

async function refreshEditorPatches() {
  if (!isTauri() || !store.document) return
  const generation = ++patchGeneration
  const fields = store.document.fields.filter(f => f.enabled && (f.clear.mode !== 'patch' || Boolean(f.clear.patchPath)))
  const textFields = store.document.fields.filter(f => f.enabled)
  const next: Record<string, CanvasPatch> = {}
  const nextText: Record<string, CanvasPatch> = {}
  await Promise.all([...fields.map(async field => {
    try {
      const payload = await invoke<PatchPayload>('render_clear_patch', { backgroundPath: store.document!.background.path, field })
      if (generation !== patchGeneration) return
      const { patch, url } = await loadPatch(payload)
      if (generation !== patchGeneration) { URL.revokeObjectURL(url); return }
      const old = patchUrls.get(field.id); if (old) URL.revokeObjectURL(old)
      patchUrls.set(field.id, url); next[field.id] = patch
    } catch (error) { feedback.reportError('背景清除预览失败', error) }
  }), ...textFields.map(async field => {
    try {
      const payload = await invoke<PatchPayload>('render_text_patch', { field, value: previewText(field) })
      if (generation !== patchGeneration) return
      const { patch, url } = await loadPatch(payload)
      if (generation !== patchGeneration) { URL.revokeObjectURL(url); return }
      const old = textPatchUrls.get(field.id); if (old) URL.revokeObjectURL(old)
      textPatchUrls.set(field.id, url); nextText[field.id] = patch
    } catch (error) { feedback.reportError('文字预览失败', error) }
  })])
  if (generation === patchGeneration) {
    const active = new Set(fields.map(f => f.id))
    for (const [id, url] of patchUrls) if (!active.has(id)) { URL.revokeObjectURL(url); patchUrls.delete(id) }
    const activeText = new Set(textFields.map(f => f.id))
    for (const [id, url] of textPatchUrls) if (!activeText.has(id)) { URL.revokeObjectURL(url); textPatchUrls.delete(id) }
    clearPatches.value = next
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
function rotation(f: TemplateField) { return f.rotationEnabled ? f.rotation : 0 }
function centeredRectConfig(f: TemplateField) {
  return { x: f.layoutRect.x + f.layoutRect.width / 2, y: f.layoutRect.y + f.layoutRect.height / 2, width: f.layoutRect.width, height: f.layoutRect.height, offsetX: f.layoutRect.width / 2, offsetY: f.layoutRect.height / 2, rotation: rotation(f) }
}
function fallbackTextConfig(f: TemplateField) {
  return { ...centeredRectConfig(f), padding: f.text.padding, text: previewText(f), fontFamily: canvasFont(f.text.fontFamily), fontSize: f.text.fontSize, fill: f.text.color, align: f.text.horizontalAlign, verticalAlign: f.text.verticalAlign, lineHeight: f.text.lineHeight, letterSpacing: f.text.letterSpacing, wrap: 'none', listening: false }
}
function setStageCursor(event: Konva.KonvaEventObject<any>, cursor: string) {
  const content = event.target.getStage()?.content as HTMLElement | undefined
  if (content) content.style.cursor = cursor
}
function beginTransform(event: Konva.KonvaEventObject<any>) {
  if (transformer.value?.getNode()?.getActiveAnchor() === 'rotater') setStageCursor(event, 'grabbing')
}
function finishTransform(id: string, event: Konva.KonvaEventObject<any>) {
  const rotated = transformer.value?.getNode()?.getActiveAnchor() === 'rotater'
  updateRect(id, event, true)
  if (rotated) setStageCursor(event, 'grab')
}
function finishMove(id: string, event: Konva.KonvaEventObject<any>) {
  updateRect(id, event, true)
  setStageCursor(event, 'move')
}
function updateRect(id: string, e: Konva.KonvaEventObject<any>, syncErase = false) {
  const f = store.document?.fields.find(x => x.id === id); if (!f) return
  const n = e.target
  const width = Math.max(8, Math.round(n.width() * Math.abs(n.scaleX())))
  const height = Math.max(8, Math.round(n.height() * Math.abs(n.scaleY())))
  const rect = { x: Math.round(n.x() - width / 2), y: Math.round(n.y() - height / 2), width, height }
  store.updateFieldTransform(id, rect, f.rotationEnabled ? n.rotation() : 0, syncErase)
  const updated = store.document?.fields.find(x => x.id === id); if (!updated) return
  const config = centeredRectConfig(updated)
  n.position({ x: config.x, y: config.y }); n.size({ width: config.width, height: config.height }); n.offset({ x: config.offsetX, y: config.offsetY }); n.rotation(config.rotation); n.scale({ x: 1, y: 1 })
  transformer.value?.getNode()?.forceUpdate()
}
</script>

<template>
  <div
    ref="host"
    class="editor-workspace relative h-full w-full overflow-auto scroll-thin"
    :class="[store.document ? 'editor-workspace-active' : 'editor-workspace-empty', store.tool === 'draw' ? 'cursor-draw' : 'cursor-select']"
  >
    <div v-if="!store.document" class="absolute inset-0 grid place-items-center">
      <button class="editor-import-card group" @click="store.importBackground">
        <span class="mb-4 grid h-11 w-11 place-items-center rounded-xl border border-primary/15 bg-primary/10 text-primary transition duration-200 group-hover:-translate-y-0.5 group-hover:bg-primary/15"><ImagePlus :size="19" /></span>
        <span class="block text-sm font-bold tracking-tight text-base-content/80">导入背景图片</span>
        <span class="mt-1.5 block text-[10px] leading-4 text-base-content/40">选择 PNG、JPG 或 JPEG 文件<br />创建可编辑的图像模板</span>
      </button>
    </div>
    <div v-else class="editor-canvas-frame m-12 inline-block" :style="{ width: `${stageSize.width}px`, height: `${stageSize.height}px` }">
      <v-stage ref="stage" :config="{ ...stageSize, scaleX: scale, scaleY: scale }" @mousedown="onDown" @mousemove="onMove" @mouseup="onUp">
        <v-layer><v-image :config="{ image, width: store.document.background.width, height: store.document.background.height, listening: false }" /></v-layer>
        <v-layer>
          <template v-for="f in store.document.fields.filter(x => x.enabled)" :key="`mask-${f.id}`">
            <v-image v-if="clearPatches[f.id]" :config="{ ...clearPatches[f.id], listening: false }" />
          </template>
        </v-layer>
        <v-layer>
          <template v-for="f in store.document.fields.filter(x => x.enabled)" :key="`text-${f.id}`">
            <v-image v-if="textPatches[f.id]" :config="{ ...textPatches[f.id], listening: false }" />
            <v-text v-else :config="fallbackTextConfig(f)" />
          </template>
        </v-layer>
        <v-layer>
          <v-rect v-for="f in store.document.fields" :key="f.id" :config="{ id: `field-${f.id}`, ...centeredRectConfig(f), stroke: f.id === store.selectedId ? '#0d99ff' : 'rgba(13,153,255,.72)', strokeWidth: 1 / scale, dash: f.id === store.selectedId ? [] : [5 / scale, 4 / scale], fill: f.id === store.selectedId ? 'rgba(13,153,255,.06)' : 'transparent', draggable: store.tool === 'select', listening: store.tool === 'select' }" @click="store.selectedId = f.id" @mouseenter="setStageCursor($event, 'move')" @mouseleave="setStageCursor($event, '')" @dragstart="setStageCursor($event, 'grabbing')" @dragend="finishMove(f.id, $event)" @transformstart="beginTransform" @transformend="finishTransform(f.id, $event)" />
          <v-rect v-if="drawing" :config="{ ...draft, stroke: '#0d99ff', strokeWidth: 1 / scale, dash: [5 / scale, 3 / scale], fill: 'rgba(13,153,255,.08)', listening: false }" />
          <v-transformer ref="transformer" :config="{ rotateEnabled: Boolean(store.selected?.rotationEnabled), rotateAnchorOffset: 26 / scale, rotateAnchorCursor: 'grab', flipEnabled: false, borderStroke: '#0d99ff', borderStrokeWidth: 1 / scale, anchorStroke: '#0d99ff', anchorStrokeWidth: 1.5 / scale, anchorFill: '#ffffff', anchorSize: 9 / scale, anchorCornerRadius: 999, keepRatio: false, enabledAnchors: ['top-left','top-center','top-right','middle-left','middle-right','bottom-left','bottom-center','bottom-right'], boundBoxFunc: (_old: unknown, next: any) => Math.abs(next.width) < 8 || Math.abs(next.height) < 8 ? _old : next }" />
        </v-layer>
      </v-stage>
    </div>
  </div>
</template>
