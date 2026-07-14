<script setup lang="ts">
import { Database, FolderOutput, Play, X } from '@lucide/vue'
import { useExportStore } from '../stores/export'
import { useTemplateStore } from '../stores/template'
defineProps<{ open: boolean }>(); defineEmits<{ close: [] }>()
const template = useTemplateStore()
const exportStore = useExportStore()
const short = (p: string) => p ? p.split(/[\\/]/).pop() : ''
</script>
<template>
  <dialog
    v-if="open"
    open
    class="modal modal-open modal-bottom absolute! inset-0! z-40 h-full! w-full! max-h-none! max-w-none! bg-base-content/15! backdrop-blur-[2px]"
    aria-label="批量生成"
    @cancel.prevent="$emit('close')"
  >
    <div class="modal-box w-full max-w-none overflow-hidden rounded-t-2xl rounded-b-none border border-b-0 border-base-content/10 bg-base-100/97 p-0 shadow-[0_-18px_60px_rgba(0,0,0,.22)] backdrop-blur-xl">
      <div class="mx-auto mt-2 h-1 w-9 rounded-full bg-base-content/12" aria-hidden="true" />

      <header class="flex h-12 items-center gap-3 border-b border-base-content/8 px-5">
        <span class="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-primary/20 bg-primary/10 text-primary"><Play :size="14" fill="currentColor" /></span>
        <div class="min-w-0">
          <h2 class="text-xs font-bold tracking-tight text-base-content/85">批量铸造</h2>
          <p class="mt-0.5 text-[9px] text-base-content/38">配置数据源与输出位置，按 CSV 行生成图像</p>
        </div>
        <span class="badge badge-ghost badge-sm ml-1 border-base-content/10 bg-base-200/55 font-mono text-[9px] text-base-content/50">{{ template.document?.fields.length ?? 0 }} 个字段</span>
        <button class="btn btn-ghost btn-sm btn-circle ml-auto h-8 min-h-8 w-8 inline-flex! text-base-content/45 hover:bg-base-200 hover:text-base-content" aria-label="关闭批量生成" @click="$emit('close')"><X :size="15"/></button>
      </header>

      <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_7rem_9rem] items-center gap-3 px-5 py-3.5">
        <button
          class="group flex h-11 min-h-11 min-w-0 items-center gap-2.5 rounded-xl border border-base-content/10 bg-base-200/45 px-3 text-left shadow-none transition duration-150 hover:border-accent/35 hover:bg-base-200/75 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
          :title="exportStore.csvPath || '选择 CSV 数据文件'"
          @click="exportStore.chooseCsv"
        >
          <span class="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-accent/10 text-accent transition group-hover:bg-accent/15"><Database :size="14" /></span>
          <span class="min-w-0 flex-1"><span class="block text-[9px] font-medium text-base-content/38">数据源 · 可选</span><span class="mt-0.5 block truncate text-[11px] font-semibold text-base-content/75">{{ short(exportStore.csvPath) || '选择 CSV 文件' }}</span></span>
          <span class="status status-xs shrink-0" :class="exportStore.csvPath ? 'status-info' : 'status-neutral'" aria-hidden="true" />
        </button>

        <button
          class="group flex h-11 min-h-11 min-w-0 items-center gap-2.5 rounded-xl border bg-base-200/45 px-3 text-left shadow-none transition duration-150 hover:bg-base-200/75 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          :class="exportStore.outputDir ? 'border-base-content/10 hover:border-primary/35' : 'border-warning/25 hover:border-warning/45'"
          :title="exportStore.outputDir || '选择图片输出目录'"
          @click="exportStore.chooseOutput"
        >
          <span class="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary transition group-hover:bg-primary/15"><FolderOutput :size="14" /></span>
          <span class="min-w-0 flex-1"><span class="block text-[9px] font-medium text-base-content/38">输出目录 · 必选</span><span class="mt-0.5 block truncate text-[11px] font-semibold text-base-content/75">{{ short(exportStore.outputDir) || '选择输出文件夹' }}</span></span>
          <span class="status status-xs shrink-0" :class="exportStore.outputDir ? 'status-success' : 'status-warning'" aria-hidden="true" />
        </button>

        <select v-model="exportStore.outputFormat" aria-label="输出格式" class="select h-11 min-h-11 w-full rounded-xl border-base-content/10 bg-base-200/45 px-3 text-xs font-bold uppercase shadow-none focus:border-primary focus:outline-none">
          <option value="png">PNG</option>
          <option value="jpg">JPG</option>
        </select>

        <button class="btn btn-primary h-11 min-h-11 w-full rounded-xl border-0 px-4 text-xs font-bold shadow-none" :disabled="exportStore.busy || !template.document" @click="exportStore.generate">
          <span v-if="exportStore.busy" class="loading loading-spinner loading-xs"/>
          <Play v-else :size="14" fill="currentColor" />
          {{ exportStore.busy ? '生成中' : '开始生成' }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button aria-label="关闭批量生成" @click.prevent="$emit('close')">close</button></form>
  </dialog>
</template>
