<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { BoxSelect, Eye, FileImage, FolderOpen, Grid2X2, MousePointer2, Play, Save, ZoomIn, ZoomOut } from '@lucide/vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import appIcon from '../src-tauri/icons/128x128.png'
import EditorCanvas from './components/EditorCanvas.vue'
import FieldList from './components/FieldList.vue'
import PropertyPanel from './components/PropertyPanel.vue'
import BatchDrawer from './components/BatchDrawer.vue'
import PreviewModal from './components/PreviewModal.vue'
import GenerationToast from './components/GenerationToast.vue'
import ThemeController from './components/ThemeController.vue'
import UnsavedChangesDialog from './components/UnsavedChangesDialog.vue'
import { useFeedbackStore } from './stores/feedback'
import { usePreviewStore } from './stores/preview'
import { useTemplateStore } from './stores/template'
import { useUnsavedChangesStore } from './stores/unsavedChanges'
const store = useTemplateStore(); const preview = usePreviewStore(); const feedback = useFeedbackStore(); const unsaved = useUnsavedChangesStore(); const batchOpen = ref(false)
let unlistenClose: (() => void) | undefined
let unlistenExit: (() => void) | undefined
const statusDotClass = computed(() => ({
  error: 'status-error ring-1 ring-error/35',
  warning: 'status-warning ring-1 ring-warning/35',
  working: 'status-info animate-pulse ring-1 ring-info/35',
  success: 'status-success ring-1 ring-success/35',
  idle: 'bg-base-content/70! text-base-content/70! ring-1 ring-base-content/35',
}[feedback.noticeType]))
const statusLabel = computed(() => ({ error: '错误', warning: '提醒', working: '处理中', success: '成功', idle: '就绪' }[feedback.noticeType]))
const editingText = (target: EventTarget | null) => target instanceof HTMLElement && (['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName) || target.isContentEditable)
const steppedZoom = (value: number, delta: number, min: number) => Math.min(4, Math.max(min, Math.round((value + delta) * 100) / 100))
function adjustCanvasZoom(delta: number) { store.zoom = steppedZoom(store.zoom, delta, .1) }
function adjustPreviewZoom(delta: number) { preview.previewZoom = steppedZoom(preview.previewZoom, delta, .05) }
async function confirmApplicationClose() {
  if (!store.dirty) return true
  const choice = await unsaved.confirm({
    title: '保存更改后再关闭应用？',
    message: `“${store.document?.name ?? '当前模板'}”包含尚未保存的修改。关闭应用后，这些修改将无法恢复。`,
    continueLabel: '关闭',
  })
  return choice !== 'cancel'
}
function onShortcut(event: KeyboardEvent) {
  if (unsaved.request) return
  if (editingText(event.target)) return
  const key = event.key.toLowerCase(); const command = event.metaKey || event.ctrlKey
  const zoomDirection = key === '+' || event.code === 'NumpadAdd' ? 1 : key === '-' || event.code === 'NumpadSubtract' ? -1 : 0
  if (zoomDirection && !command && !event.altKey) {
    event.preventDefault()
    if (preview.previewOpen) adjustPreviewZoom(zoomDirection * .1)
    else adjustCanvasZoom(zoomDirection * .1)
    return
  }
  if (preview.previewOpen) { if (key === 'escape') { event.preventDefault(); preview.closePreview() }; return }
  if (command && key === 'd') { event.preventDefault(); if (!event.repeat) store.duplicateSelected(); return }
  if (command && key === 's') { event.preventDefault(); if (!event.repeat) store.saveTemplate(); return }
  if (command || event.altKey) return
  if (key === 'v') { event.preventDefault(); store.tool = 'select'; return }
  if ((key === 'f' || key === 'r') && store.document) { event.preventDefault(); store.tool = 'draw'; return }
  if (key === 'escape') { event.preventDefault(); if (store.tool === 'draw') store.tool = 'select'; else store.selectedId = null; return }
  if ((key === 'delete' || key === 'backspace') && store.selected) { event.preventDefault(); store.removeSelected(); return }
  const directions: Record<string, [number, number]> = { arrowleft: [-1, 0], arrowright: [1, 0], arrowup: [0, -1], arrowdown: [0, 1] }
  if (directions[key] && store.tool === 'select' && store.selected) { event.preventDefault(); const [x, y] = directions[key]; const step = event.shiftKey ? 10 : 1; store.nudgeSelected(x * step, y * step) }
}
onMounted(async () => {
  window.addEventListener('keydown', onShortcut)
  if (!isTauri()) return
  const appWindow = getCurrentWindow()
  try {
    unlistenClose = await appWindow.onCloseRequested(async event => {
      if (!store.dirty) return
      event.preventDefault()
      if (await confirmApplicationClose()) {
        try { await appWindow.destroy() }
        catch (error) { feedback.reportError('关闭应用失败', error) }
      }
    })
    unlistenExit = await appWindow.listen('app-exit-requested', async () => {
      if (await confirmApplicationClose()) {
        try { await invoke('confirm_app_exit') }
        catch (error) { feedback.reportError('退出应用失败', error) }
      }
    })
  } catch (error) { feedback.reportError('无法启用未保存更改保护', error) }
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onShortcut)
  unlistenClose?.()
  unlistenExit?.()
})
</script>
<template>
  <main class="flex h-screen flex-col overflow-hidden bg-base-100 text-base-content">
    <header class="flex h-14 shrink-0 items-center border-b border-base-content/10 bg-base-100/95 px-4 backdrop-blur-xl">
      <div class="mr-5 flex items-center gap-2"><img :src="appIcon" alt="" class="h-7 w-7 shrink-0 object-contain" /><div><h1 class="text-xs font-black tracking-[.14em]">像素铸坊</h1><p class="text-[8px] uppercase tracking-[.2em] text-base-content/35">Pixel Forge</p></div></div>
      <div class="flex gap-1"><button class="tool-btn" @click="store.importBackground"><FileImage :size="14" />导入图片</button><button class="tool-btn" @click="store.openTemplate"><FolderOpen :size="14" />打开模板</button><button class="tool-btn" :disabled="!store.document" @click="store.saveTemplate"><Save :size="14" />保存模板<span v-if="store.dirty" class="h-1.5 w-1.5 rounded-full bg-secondary" /></button></div>
      <div class="mx-3 h-5 w-px bg-base-content/10" />
      <div class="join rounded-lg border border-base-content/10 bg-base-200/60 p-0.5"><button class="btn btn-sm join-item h-8 min-h-0 px-2.5 text-xs font-semibold shadow-none" :class="store.tool === 'select' ? 'btn-primary' : 'btn-ghost text-base-content/50'" :aria-pressed="store.tool === 'select'" title="选择工具（V）" @click="store.tool = 'select'"><MousePointer2 :size="14"/>选择<kbd class="kbd kbd-xs ml-1" :class="store.tool === 'select' ? 'border-primary-content/20! bg-primary-content/20! text-primary-content!' : 'border-base-300! bg-base-100! text-base-content!'">V</kbd></button><button class="btn btn-sm join-item h-8 min-h-0 px-2.5 text-xs font-semibold shadow-none" :class="store.tool === 'draw' ? 'btn-primary' : 'btn-ghost text-base-content/50'" :aria-pressed="store.tool === 'draw'" title="画框工具（F 或 R）" @click="store.tool = 'draw'"><BoxSelect :size="14"/>画框<kbd class="kbd kbd-xs ml-1" :class="store.tool === 'draw' ? 'border-primary-content/20! bg-primary-content/20! text-primary-content!' : 'border-base-300! bg-base-100! text-base-content!'">F</kbd></button></div>
      <div class="ml-auto flex min-w-0 items-center gap-1.5"><span class="max-w-40 truncate rounded-full bg-base-200/60 px-2.5 py-1.5 font-mono text-[10px] text-base-content/40 xl:max-w-64">{{ store.document?.name ?? '未打开模板' }}</span><ThemeController/><button class="tool-btn" :disabled="!store.document || preview.previewBusy" title="预览最终导出效果" @click="preview.createPreview"><span v-if="preview.previewBusy" class="loading loading-spinner loading-xs"/><Eye v-else :size="14"/>预览</button><button class="btn btn-primary btn-sm h-8 min-h-0 rounded-md px-3.5 text-xs shadow-none" :disabled="!store.document" @click="batchOpen = true"><Play :size="13" fill="currentColor"/>批量生成</button></div>
    </header>
    <div class="relative flex min-h-0 flex-1">
      <FieldList />
      <section class="relative min-w-0 flex-1"><EditorCanvas />
        <div class="pointer-events-none absolute left-3 top-3 flex items-center gap-2 rounded-md border px-2.5 py-1.5 text-[10px] font-bold uppercase tracking-[.14em] shadow-lg backdrop-blur" :class="store.tool === 'draw' ? 'border-success/45 bg-neutral/90 text-success' : 'border-accent/30 bg-neutral/90 text-accent'"><component :is="store.tool === 'draw' ? BoxSelect : MousePointer2" :size="13"/><span>{{ store.tool === 'draw' ? '画框模式 · 拖动创建区域' : '选择模式 · 点击或移动画框' }}</span></div>
        <div class="absolute bottom-3 left-1/2 flex -translate-x-1/2 items-center gap-0.5 rounded-full border border-base-content/10 bg-base-100/90 p-1 shadow-[0_8px_28px_rgba(0,0,0,.22)] backdrop-blur-xl">
          <button class="tooltip tooltip-top btn btn-ghost btn-xs btn-circle h-7 w-7 min-h-0 inline-flex! text-base-content/55 hover:text-base-content" data-tip="缩小（-）" aria-label="缩小画布" @click="adjustCanvasZoom(-.1)"><ZoomOut :size="14"/></button>
          <span class="w-14 select-none text-center font-mono text-[11px] font-semibold tabular-nums text-base-content/70">{{ Math.round(store.zoom * 100) }}%</span>
          <button class="tooltip tooltip-top btn btn-ghost btn-xs btn-circle h-7 w-7 min-h-0 inline-flex! text-base-content/55 hover:text-base-content" data-tip="放大（+）" aria-label="放大画布" @click="adjustCanvasZoom(.1)"><ZoomIn :size="14"/></button>
          <span class="mx-1 h-4 w-px bg-base-content/12" aria-hidden="true" />
          <button class="btn btn-ghost btn-xs h-7 min-h-0 rounded-full px-2.5 text-base-content/65 hover:text-base-content" aria-label="恢复 1:1 缩放" @click="store.zoom = 1"><Grid2X2 :size="14"/><span class="font-mono text-[11px] font-semibold">1:1</span></button>
        </div>
      </section>
      <PropertyPanel />
      <BatchDrawer :open="batchOpen" @close="batchOpen = false" />
      <PreviewModal />
      <GenerationToast />
      <UnsavedChangesDialog />
    </div>
    <footer role="status" aria-live="polite" class="flex h-6 shrink-0 items-center border-t border-base-300 bg-neutral px-3 font-mono text-[9px] text-base-content/35"><span class="status status-sm mr-2 shrink-0" :class="statusDotClass" :aria-label="statusLabel"/><span class="min-w-0 truncate font-medium">{{ feedback.notice }}</span><span class="ml-auto shrink-0 pl-4 opacity-60">{{ store.document ? `${store.document.background.width} × ${store.document.background.height} px · ${store.document.fields.length} 字段` : '等待导入' }}</span></footer>
  </main>
</template>
