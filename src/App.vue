<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { BoxSelect, ChevronDown, Eye, FileImage, FolderOpen, Grid2X2, MousePointer2, Play, Save, ZoomIn, ZoomOut } from '@lucide/vue'
import EditorCanvas from './components/EditorCanvas.vue'
import FieldList from './components/FieldList.vue'
import PropertyPanel from './components/PropertyPanel.vue'
import BatchDrawer from './components/BatchDrawer.vue'
import PreviewModal from './components/PreviewModal.vue'
import GenerationToast from './components/GenerationToast.vue'
import { useTemplateStore } from './stores/template'
const store = useTemplateStore(); const batchOpen = ref(false)
const editingText = (target: EventTarget | null) => target instanceof HTMLElement && (['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName) || target.isContentEditable)
function onShortcut(event: KeyboardEvent) {
  if (editingText(event.target)) return
  const key = event.key.toLowerCase(); const command = event.metaKey || event.ctrlKey
  if (store.previewOpen) { if (key === 'escape') { event.preventDefault(); store.closePreview() }; return }
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
onMounted(() => window.addEventListener('keydown', onShortcut))
onBeforeUnmount(() => window.removeEventListener('keydown', onShortcut))
</script>
<template>
  <main data-theme="forge" class="flex h-screen flex-col overflow-hidden bg-base-100 text-base-content">
    <header class="flex h-14 shrink-0 items-center border-b border-base-300 bg-neutral px-3">
      <div class="mr-6 flex items-center gap-2 px-2"><div class="grid h-7 w-7 place-items-center bg-primary text-sm font-black text-primary-content">P</div><div><h1 class="text-xs font-black tracking-[.16em]">像素铸坊</h1><p class="text-[8px] uppercase tracking-[.22em] text-base-content/35">Image Template Studio</p></div></div>
      <div class="flex gap-1"><button class="tool-btn" @click="store.importBackground"><FileImage :size="14" />导入图片</button><button class="tool-btn" @click="store.openTemplate"><FolderOpen :size="14" />打开模板</button><button class="tool-btn" :disabled="!store.document" @click="store.saveTemplate"><Save :size="14" />保存模板<span v-if="store.dirty" class="h-1.5 w-1.5 rounded-full bg-secondary" /></button></div>
      <div class="mx-4 h-6 w-px bg-base-300" />
      <div class="flex overflow-hidden rounded-md border border-base-300 bg-base-200 p-0.5"><button class="btn btn-xs h-8 rounded px-3 font-semibold" :class="store.tool === 'select' ? 'btn-primary' : 'btn-ghost text-base-content/45'" :aria-pressed="store.tool === 'select'" title="选择工具（V）" @click="store.tool = 'select'"><MousePointer2 :size="14"/>选择<kbd class="kbd kbd-xs ml-1" :class="store.tool === 'select' ? '!border-primary-content/20 !bg-primary-content/20 !text-primary-content' : '!border-base-300 !bg-base-100 !text-base-content'">V</kbd></button><button class="btn btn-xs h-8 rounded px-3 font-semibold" :class="store.tool === 'draw' ? 'btn-primary' : 'btn-ghost text-base-content/45'" :aria-pressed="store.tool === 'draw'" title="画框工具（F 或 R）" @click="store.tool = 'draw'"><BoxSelect :size="14"/>画框<kbd class="kbd kbd-xs ml-1" :class="store.tool === 'draw' ? '!border-primary-content/20 !bg-primary-content/20 !text-primary-content' : '!border-base-300 !bg-base-100 !text-base-content'">F</kbd></button></div>
      <div class="ml-auto flex items-center gap-2"><span class="max-w-52 truncate font-mono text-[10px] text-base-content/35">{{ store.document?.name ?? '未打开模板' }}</span><button class="btn btn-ghost btn-sm rounded-md" :disabled="!store.document || store.previewBusy" title="预览最终导出效果" @click="store.createPreview"><span v-if="store.previewBusy" class="loading loading-spinner loading-xs"/><Eye v-else :size="14"/>预览</button><button class="btn btn-primary btn-sm rounded-md px-4" :disabled="!store.document" @click="batchOpen = true"><Play :size="13" fill="currentColor"/>批量生成</button></div>
    </header>
    <div class="relative flex min-h-0 flex-1">
      <FieldList />
      <section class="relative min-w-0 flex-1"><EditorCanvas />
        <div class="pointer-events-none absolute left-3 top-3 flex items-center gap-2 rounded-md border px-2.5 py-1.5 text-[10px] font-bold uppercase tracking-[.14em] shadow-lg backdrop-blur" :class="store.tool === 'draw' ? 'border-primary/50 bg-primary text-primary-content' : 'border-accent/30 bg-neutral/90 text-accent'"><component :is="store.tool === 'draw' ? BoxSelect : MousePointer2" :size="13"/><span>{{ store.tool === 'draw' ? '画框模式 · 拖动创建区域' : '选择模式 · 点击或移动画框' }}</span></div>
        <div class="absolute bottom-3 left-1/2 flex -translate-x-1/2 items-center gap-1 rounded-lg border border-base-300 bg-neutral/90 p-1 shadow-xl backdrop-blur">
          <button class="btn btn-ghost btn-xs" @click="store.zoom = Math.max(.1, store.zoom - .1)"><ZoomOut :size="13"/></button><span class="w-12 text-center font-mono text-[10px]">{{ Math.round(store.zoom * 100) }}%</span><button class="btn btn-ghost btn-xs" @click="store.zoom = Math.min(4, store.zoom + .1)"><ZoomIn :size="13"/></button><div class="mx-1 h-4 w-px bg-base-300"/><button class="btn btn-ghost btn-xs" @click="store.zoom = 1"><Grid2X2 :size="13"/>1:1</button>
        </div>
      </section>
      <PropertyPanel />
      <BatchDrawer :open="batchOpen" @close="batchOpen = false" />
      <PreviewModal />
      <GenerationToast />
    </div>
    <footer class="flex h-6 shrink-0 items-center border-t border-base-300 bg-neutral px-3 font-mono text-[9px] text-base-content/35"><span class="mr-2 h-1.5 w-1.5 rounded-full" :class="store.notice.startsWith('生成失败') ? 'bg-error' : 'bg-success'"/>{{ store.notice }}<span class="ml-auto">{{ store.document ? `${store.document.background.width} × ${store.document.background.height} px · ${store.document.fields.length} 字段` : '等待导入' }}</span></footer>
  </main>
</template>
