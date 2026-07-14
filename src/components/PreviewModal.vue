<script setup lang="ts">
import { Grid2X2, ImageIcon, RefreshCw, X, ZoomIn, ZoomOut } from '@lucide/vue'
import { useExportStore } from '../stores/export'
import { usePreviewStore } from '../stores/preview'
import { useTemplateStore } from '../stores/template'
const template = useTemplateStore()
const preview = usePreviewStore()
const exportStore = useExportStore()
</script>

<template>
  <dialog v-if="preview.previewOpen" open class="modal modal-open z-50" @cancel.prevent="preview.closePreview">
    <div class="modal-box flex max-h-[92vh] w-11/12 max-w-5xl flex-col overflow-hidden border border-base-300 bg-base-200 p-0 shadow-2xl">
      <header class="flex h-14 shrink-0 items-center gap-3 border-b border-base-300 bg-neutral px-5">
        <span class="grid h-8 w-8 place-items-center rounded-md bg-primary/15 text-primary"><ImageIcon :size="16" /></span>
        <div><h2 class="text-sm font-bold">图像修改预览</h2><p class="text-[10px] text-base-content/40">{{ exportStore.csvPath ? 'CSV 第一行' : '模板当前值' }} · {{ exportStore.outputFormat.toUpperCase() }} · 拖动分割线对比</p></div>
        <div class="ml-auto flex items-center gap-0.5 rounded-full border border-base-content/10 bg-base-100/70 p-1">
          <button class="tooltip tooltip-bottom btn btn-ghost btn-xs btn-circle h-7 w-7 min-h-0 inline-flex! text-base-content/55 hover:text-base-content" data-tip="缩小（-）" aria-label="缩小预览" @click="preview.previewZoom = Math.max(.05, preview.previewZoom - .1)"><ZoomOut :size="14" /></button>
          <span class="w-14 select-none text-center font-mono text-[11px] font-semibold tabular-nums text-base-content/70">{{ Math.round(preview.previewZoom * 100) }}%</span>
          <button class="tooltip tooltip-bottom btn btn-ghost btn-xs btn-circle h-7 w-7 min-h-0 inline-flex! text-base-content/55 hover:text-base-content" data-tip="放大（+）" aria-label="放大预览" @click="preview.previewZoom = Math.min(4, preview.previewZoom + .1)"><ZoomIn :size="14" /></button>
          <span class="mx-1 h-4 w-px bg-base-content/12" aria-hidden="true" />
          <button class="btn btn-ghost btn-xs h-7 min-h-0 rounded-full px-2.5 text-base-content/65 hover:text-base-content" aria-label="恢复 1:1 缩放" @click="preview.previewZoom = 1"><Grid2X2 :size="14" /><span class="font-mono text-[11px] font-semibold">1:1</span></button>
        </div>
        <span v-if="template.document" class="badge badge-ghost h-9 rounded-full border-base-content/10 bg-base-100/70 px-3 font-mono text-[10px] font-medium tabular-nums text-base-content/55">{{ template.document.background.width }} × {{ template.document.background.height }} px</span>
        <button class="btn btn-ghost btn-sm btn-circle inline-flex!" aria-label="关闭预览" @click="preview.closePreview"><X :size="16" /></button>
      </header>
      <div class="checkerboard scroll-thin min-h-0 flex-1 overflow-auto">
        <div v-if="preview.previewBusy" class="flex min-h-80 flex-col items-center justify-center gap-4 text-base-content/45"><span class="loading loading-spinner loading-lg text-primary"/><span class="text-xs">正在执行背景修补与文字渲染…</span></div>
        <div v-else-if="preview.previewUrl && template.document" class="inline-flex min-h-full min-w-full items-center justify-center p-8">
          <div
            class="relative max-w-none flex-none shadow-2xl shadow-black/50"
            :style="{ width: `${template.document.background.width * preview.previewZoom}px`, height: `${template.document.background.height * preview.previewZoom}px` }"
          >
            <figure class="diff h-full w-full bg-white" tabindex="0" aria-label="原图与生成结果对比">
              <div class="diff-item-1" role="img" aria-label="生成结果" tabindex="0">
                <img :src="preview.previewUrl" alt="生成结果" draggable="false" />
              </div>
              <div class="diff-item-2" role="img" aria-label="原图">
                <img :src="template.backgroundUrl" alt="原图" draggable="false" />
              </div>
              <div class="diff-resizer" aria-hidden="true" />
            </figure>
            <div class="pointer-events-none absolute inset-x-0 top-0 z-10 flex items-start justify-between p-2">
              <span class="badge badge-neutral badge-sm border-base-content/15 bg-neutral/85 text-[10px] shadow-sm backdrop-blur">生成结果</span>
              <span class="badge badge-neutral badge-sm border-base-content/15 bg-neutral/85 text-[10px] shadow-sm backdrop-blur">原图</span>
            </div>
          </div>
        </div>
        <div v-else class="grid min-h-80 place-items-center text-sm text-error">预览生成失败，请检查状态栏提示</div>
      </div>
      <footer class="flex shrink-0 items-center border-t border-base-300 bg-neutral px-5 py-3"><p class="text-[10px] text-base-content/35">随机字段每次渲染会重新取值，预览结果可能与批量文件不同。</p><button class="btn btn-ghost btn-sm ml-auto" :disabled="preview.previewBusy" @click="preview.createPreview"><RefreshCw :size="14" />重新渲染</button><button class="btn btn-primary btn-sm ml-2 px-5" @click="preview.closePreview">完成</button></footer>
    </div>
    <form method="dialog" class="modal-backdrop"><button aria-label="关闭预览" @click.prevent="preview.closePreview">close</button></form>
  </dialog>
</template>
