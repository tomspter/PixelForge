<script setup lang="ts">
import { Copy, Eye, EyeOff, GripVertical, Plus, Trash2 } from '@lucide/vue'
import { useTemplateStore } from '../stores/template'
const store = useTemplateStore()
</script>
<template>
  <aside class="flex min-h-0 w-60 shrink-0 flex-col border-r border-base-300 bg-base-200">
    <div class="flex h-12 items-center justify-between border-b border-base-300 px-4">
      <span class="panel-title">字段图层</span>
      <span class="rounded bg-base-300 px-1.5 py-0.5 font-mono text-[10px] text-base-content/50">{{ store.document?.fields.length ?? 0 }}</span>
    </div>
    <div class="scroll-thin min-h-0 flex-1 overflow-y-auto p-2">
      <div v-if="!store.document?.fields.length" class="px-3 py-10 text-center text-xs leading-6 text-base-content/35">选择上方“画框”工具<br />在画布上拖出字段区域</div>
      <button v-for="(f, i) in store.document?.fields" :key="f.id" class="group mb-1 flex w-full items-center gap-2 rounded-md border px-2 py-2 text-left transition" :class="f.id === store.selectedId ? 'border-primary/40 bg-primary/10' : 'border-transparent hover:bg-base-300'" @click="store.selectedId = f.id">
        <GripVertical :size="13" class="text-base-content/20" />
        <span class="grid h-6 w-6 place-items-center rounded text-[10px] font-bold" :class="f.id === store.selectedId ? 'bg-primary text-primary-content' : 'bg-base-300'">{{ String(i + 1).padStart(2, '0') }}</span>
        <span class="min-w-0 flex-1"><span class="block truncate text-xs font-semibold">{{ f.name }}</span><span class="block font-mono text-[9px] text-base-content/35">{{ f.layoutRect.width }} × {{ f.layoutRect.height }}</span></span>
        <component :is="f.enabled ? Eye : EyeOff" :size="13" class="text-base-content/35" @click.stop="f.enabled = !f.enabled; store.markDirty()" />
      </button>
    </div>
    <div class="grid grid-cols-3 gap-1 border-t border-base-300 p-2">
      <button class="btn btn-ghost btn-xs" title="新建画框" @click="store.tool = 'draw'"><Plus :size="14" /></button>
      <button class="btn btn-ghost btn-xs" title="复制字段（⌘/Ctrl + D）" :disabled="!store.selected" @click="store.duplicateSelected"><Copy :size="13" /></button>
      <button class="btn btn-ghost btn-xs text-error" title="删除字段" :disabled="!store.selected" @click="store.removeSelected"><Trash2 :size="13" /></button>
    </div>
  </aside>
</template>
