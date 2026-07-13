<script setup lang="ts">
import { Database, FolderOutput, Play, X } from '@lucide/vue'
import { useTemplateStore } from '../stores/template'
defineProps<{ open: boolean }>(); defineEmits<{ close: [] }>()
const store = useTemplateStore()
const short = (p: string) => p ? p.split(/[\\/]/).pop() : ''
</script>
<template>
  <div v-if="open" class="absolute inset-x-0 bottom-0 z-30 border-t border-primary/30 bg-neutral/95 shadow-[0_-20px_60px_rgba(0,0,0,.45)] backdrop-blur">
    <div class="flex items-center gap-5 px-6 py-4">
      <div class="mr-2"><span class="panel-title text-primary/70">批量铸造</span><p class="mt-1 text-xs text-base-content/40">CSV 每行生成一张图片；无 CSV 时生成单张预览。</p></div>
      <button class="flex min-w-56 items-center gap-3 rounded-lg border border-base-300 bg-base-200 px-4 py-3 text-left hover:border-accent/50" @click="store.chooseCsv"><Database :size="18" class="text-accent"/><span><span class="block text-[10px] text-base-content/35">数据源 · 可选</span><span class="block max-w-40 truncate text-xs font-semibold">{{ short(store.csvPath) || '选择 CSV 文件' }}</span></span></button>
      <button class="flex min-w-56 items-center gap-3 rounded-lg border border-base-300 bg-base-200 px-4 py-3 text-left hover:border-primary/50" @click="store.chooseOutput"><FolderOutput :size="18" class="text-primary"/><span><span class="block text-[10px] text-base-content/35">输出目录 · 必选</span><span class="block max-w-40 truncate text-xs font-semibold">{{ short(store.outputDir) || '选择文件夹' }}</span></span></button>
      <select v-model="store.outputFormat" class="select select-sm select-bordered w-24 rounded-md bg-base-200 text-xs font-bold uppercase"><option value="png">PNG</option><option value="jpg">JPG</option></select>
      <button class="btn btn-primary ml-auto min-w-32 rounded-md" :disabled="store.busy || !store.document" @click="store.generate"><span v-if="store.busy" class="loading loading-spinner loading-xs"/><Play v-else :size="15" fill="currentColor" />{{ store.busy ? '生成中' : '开始生成' }}</button>
      <button class="btn btn-ghost btn-sm btn-circle" @click="$emit('close')"><X :size="16"/></button>
    </div>
  </div>
</template>
