<script setup lang="ts">
import { computed, ref } from 'vue'
import { Copy, Eye, EyeOff, GripVertical, Plus, Trash2 } from '@lucide/vue'
import { useDraggable, type DraggableEvent } from 'vue-draggable-plus'
import { useTemplateStore } from '../stores/template'
import type { TemplateField } from '../types/template'
const store = useTemplateStore()
const fieldList = ref<HTMLElement | null>(null)
const sortAnnouncement = ref('')
const visualFields = computed<TemplateField[]>(() => [...(store.document?.fields ?? [])].reverse())
const commitVisualOrder = (fields: TemplateField[]) => store.reorderFields([...fields].reverse())
const onEnabledChange = (id: string, event: Event) => store.setFieldEnabled(id, (event.currentTarget as HTMLInputElement).checked)
function moveWithKeyboard(field: TemplateField, direction: -1 | 1, event: KeyboardEvent) {
  if (!event.altKey) return
  event.preventDefault()
  const fields = [...visualFields.value]
  const index = fields.findIndex(candidate => candidate.id === field.id)
  const nextIndex = index + direction
  if (index < 0 || nextIndex < 0 || nextIndex >= fields.length) return
  fields.splice(nextIndex, 0, fields.splice(index, 1)[0])
  commitVisualOrder(fields)
  store.selectedId = field.id
  sortAnnouncement.value = `${field.name} 已移动到第 ${nextIndex + 1} 层`
}
const draggedId = (event: DraggableEvent<TemplateField>) => event.item.dataset.fieldId ?? null
function commitDragOrder(event: DraggableEvent<TemplateField>) {
  const from = event.oldDraggableIndex ?? event.oldIndex
  const to = event.newDraggableIndex ?? event.newIndex
  if (from == null || to == null || from === to) return
  const fields = [...visualFields.value]
  const [moved] = fields.splice(from, 1)
  if (!moved) return
  fields.splice(to, 0, moved)
  commitVisualOrder(fields)
  sortAnnouncement.value = `${moved.name} 已移动到第 ${to + 1} 层`
}
useDraggable<TemplateField>(fieldList, {
  animation: 200,
  easing: 'cubic-bezier(.2,.8,.2,1)',
  direction: 'vertical',
  draggable: '.field-layer-row',
  handle: '.field-drag-handle',
  dataIdAttr: 'data-field-id',
  chosenClass: 'field-layer-chosen',
  dragClass: 'field-layer-drag',
  ghostClass: 'field-layer-ghost',
  fallbackClass: 'field-layer-fallback',
  forceFallback: true,
  fallbackOnBody: false,
  fallbackTolerance: 4,
  scroll: true,
  bubbleScroll: true,
  scrollSensitivity: 48,
  scrollSpeed: 12,
  onStart(event) {
    const id = draggedId(event)
    if (id) store.selectedId = id
  },
  customUpdate: commitDragOrder,
})
</script>
<template>
  <aside class="flex min-h-0 w-60 shrink-0 flex-col border-r border-base-300 bg-base-200">
    <p class="sr-only" aria-live="polite">{{ sortAnnouncement }}</p>
    <div class="flex h-12 items-center justify-between border-b border-base-300 px-4">
      <span class="panel-title">字段图层</span>
      <span class="badge badge-ghost badge-sm font-mono text-[10px]">{{ store.document?.fields.length ?? 0 }}</span>
    </div>
    <div class="scroll-thin min-h-0 flex-1 overflow-y-auto p-2">
      <div v-if="!store.document?.fields.length" class="px-3 py-10 text-center text-xs leading-6 text-base-content/35">选择上方“画框”工具<br />在画布上拖出字段区域</div>
      <ul ref="fieldList" class="list gap-1" aria-label="字段图层（顶部字段最后绘制）" v-show="visualFields.length">
        <li
          v-for="(f, i) in visualFields"
          :key="f.id"
          :data-field-id="f.id"
          class="field-layer-row list-row gap-2! border p-2! text-left"
          :class="f.id === store.selectedId ? 'border-primary/40 bg-primary/10' : 'border-transparent hover:bg-base-300'"
          style="--list-grid-cols: auto auto minmax(0, 1fr) auto"
        >
          <button type="button" class="absolute inset-0 z-0 rounded-md focus-visible:outline-2 focus-visible:outline-primary" :aria-label="`选择字段 ${f.name}`" :aria-current="f.id === store.selectedId" @click="store.selectedId = f.id" />
          <button type="button" class="field-drag-handle relative z-20 grid h-6 w-4 touch-none place-items-center self-center rounded text-base-content/25 transition-colors hover:bg-base-content/8 hover:text-base-content/60 focus-visible:outline-2 focus-visible:outline-primary" :aria-label="`拖动调整 ${f.name} 的图层顺序`" aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown" title="拖动排序；Alt + ↑/↓ 键盘排序" @click.stop="store.selectedId = f.id" @keydown.up="moveWithKeyboard(f, -1, $event)" @keydown.down="moveWithKeyboard(f, 1, $event)"><GripVertical :size="13" /></button>
          <span class="pointer-events-none grid h-6 w-6 place-items-center self-center rounded text-[10px] font-bold" :class="f.id === store.selectedId ? 'bg-primary text-primary-content' : 'bg-base-300'">{{ String(i + 1).padStart(2, '0') }}</span>
          <span class="pointer-events-none min-w-0 self-center"><span class="block truncate text-xs font-semibold">{{ f.name }}</span><span class="block font-mono text-[9px] text-base-content/35">{{ f.layoutRect.width }} × {{ f.layoutRect.height }}<template v-if="f.rotationEnabled"> · {{ f.rotation }}°</template></span></span>
          <label
            class="swap tooltip tooltip-left btn btn-ghost btn-xs btn-square relative z-20 inline-grid! self-center text-base-content/35"
            :data-tip="f.enabled ? '隐藏字段' : '显示字段'"
            @click.stop
          >
            <input :checked="f.enabled" type="checkbox" :aria-label="f.enabled ? `隐藏${f.name}` : `显示${f.name}`" @change="onEnabledChange(f.id, $event)" />
            <Eye :size="13" class="swap-on" />
            <EyeOff :size="13" class="swap-off" />
          </label>
        </li>
      </ul>
    </div>
    <div class="grid grid-cols-3 gap-1 border-t border-base-300 p-2">
      <div class="tooltip tooltip-top" data-tip="新建画框"><button class="btn btn-ghost btn-xs w-full" aria-label="新建画框" @click="store.tool = 'draw'"><Plus :size="14" /></button></div>
      <div class="tooltip tooltip-top" data-tip="复制字段（⌘/Ctrl + D）"><button class="btn btn-ghost btn-xs w-full" aria-label="复制字段" :disabled="!store.selected" @click="store.duplicateSelected"><Copy :size="13" /></button></div>
      <div class="tooltip tooltip-top" data-tip="删除字段"><button class="btn btn-ghost btn-xs w-full text-error" aria-label="删除字段" :disabled="!store.selected" @click="store.removeSelected"><Trash2 :size="13" /></button></div>
    </div>
  </aside>
</template>
