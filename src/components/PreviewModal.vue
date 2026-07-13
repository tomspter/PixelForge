<script setup lang="ts">
import { Grid2X2, ImageIcon, RefreshCw, X, ZoomIn, ZoomOut } from '@lucide/vue'
import { useTemplateStore } from '../stores/template'
const store = useTemplateStore()
</script>

<template>
  <dialog v-if="store.previewOpen" open class="modal modal-open z-50" @cancel.prevent="store.closePreview">
    <div class="modal-box flex max-h-[92vh] w-11/12 max-w-5xl flex-col overflow-hidden border border-base-300 bg-base-200 p-0 shadow-2xl">
      <header class="flex h-14 shrink-0 items-center gap-3 border-b border-base-300 bg-neutral px-5">
        <span class="grid h-8 w-8 place-items-center rounded-md bg-primary/15 text-primary"><ImageIcon :size="16" /></span>
        <div><h2 class="text-sm font-bold">最终图像预览</h2><p class="text-[10px] text-base-content/40">Rust 实际导出管线 · {{ store.csvPath ? 'CSV 第一行' : '模板当前值' }} · {{ store.outputFormat.toUpperCase() }}</p></div>
        <div class="ml-auto flex items-center gap-1 rounded-md border border-base-300 bg-base-200 p-0.5"><button class="btn btn-ghost btn-xs" title="缩小" @click="store.previewZoom = Math.max(.05, store.previewZoom - .1)"><ZoomOut :size="13" /></button><span class="w-12 text-center font-mono text-[10px]">{{ Math.round(store.previewZoom * 100) }}%</span><button class="btn btn-ghost btn-xs" title="放大" @click="store.previewZoom = Math.min(4, store.previewZoom + .1)"><ZoomIn :size="13" /></button><button class="btn btn-ghost btn-xs" title="原始像素比例" @click="store.previewZoom = 1"><Grid2X2 :size="13" />1:1</button></div>
        <span v-if="store.document" class="badge badge-ghost font-mono text-[10px]">{{ store.document.background.width }} × {{ store.document.background.height }} px</span>
        <button class="btn btn-ghost btn-sm btn-circle" aria-label="关闭预览" @click="store.closePreview"><X :size="16" /></button>
      </header>
      <div class="checkerboard scroll-thin min-h-0 flex-1 overflow-auto">
        <div v-if="store.previewBusy" class="flex min-h-80 flex-col items-center justify-center gap-4 text-base-content/45"><span class="loading loading-spinner loading-lg text-primary"/><span class="text-xs">正在执行背景修补与文字渲染…</span></div>
        <div v-else-if="store.previewUrl && store.document" class="inline-flex min-h-full min-w-full items-center justify-center p-8"><img :src="store.previewUrl" alt="最终导出效果预览" class="max-w-none flex-none bg-white shadow-2xl shadow-black/50" :style="{ width: `${store.document.background.width * store.previewZoom}px`, height: `${store.document.background.height * store.previewZoom}px` }" /></div>
        <div v-else class="grid min-h-80 place-items-center text-sm text-error">预览生成失败，请检查状态栏提示</div>
      </div>
      <footer class="flex shrink-0 items-center border-t border-base-300 bg-neutral px-5 py-3"><p class="text-[10px] text-base-content/35">随机字段每次渲染会重新取值，预览结果可能与批量文件不同。</p><button class="btn btn-ghost btn-sm ml-auto" :disabled="store.previewBusy" @click="store.createPreview"><RefreshCw :size="14" />重新渲染</button><button class="btn btn-primary btn-sm ml-2 px-5" @click="store.closePreview">完成</button></footer>
    </div>
    <form method="dialog" class="modal-backdrop"><button aria-label="关闭预览" @click.prevent="store.closePreview">close</button></form>
  </dialog>
</template>
