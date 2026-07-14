<script setup lang="ts">
import { computed } from 'vue'
import { BoxSelect, Database, Eraser, RotateCcw, RotateCw, SlidersHorizontal, Type as TypeIcon } from '@lucide/vue'
import { useTemplateStore } from '../stores/template'
import type { FieldKind } from '../types/template'
import { DEFAULT_DATE_TIME_FORMAT } from '../utils/dateTime'
import DateTimeCalendar from './DateTimeCalendar.vue'
const store = useTemplateStore()
const f = computed(() => store.selected)
const kindLabel = computed(() => ({ fixed: '固定值', csv: 'CSV', random: '随机数', date: '日期时间' }[f.value?.kind ?? 'fixed']))
const clearLabel = computed(() => ({ inpaint: '智能抹除', telea: 'Telea', patch: '背景补丁' }[f.value?.clear.mode ?? 'inpaint']))
const controlId = (name: string) => `field-${f.value?.id ?? 'none'}-${name}`
function onKindChange(event: Event) {
  if (!f.value) return
  store.setFieldKind(f.value.id, (event.currentTarget as HTMLSelectElement).value as FieldKind)
}
function commitRotation(event: Event) {
  if (!f.value) return
  store.setRotation(f.value.id, Number((event.currentTarget as HTMLInputElement).value))
}
function toggleRotation(event: Event) {
  if (!f.value) return
  store.setRotationEnabled(f.value.id, (event.currentTarget as HTMLInputElement).checked)
}
</script>
<template>
  <aside class="flex min-h-0 w-80 shrink-0 flex-col border-l border-base-300 bg-base-200">
    <header class="flex h-14 shrink-0 items-center gap-3 border-b border-base-300 bg-base-200/95 px-4 backdrop-blur">
      <span class="grid h-8 w-8 shrink-0 place-items-center rounded-lg border border-base-content/10 bg-base-100 text-base-content/45"><SlidersHorizontal :size="15" /></span>
      <div class="min-w-0">
        <span class="panel-title block">属性检查器</span>
        <p class="mt-0.5 truncate text-[11px] font-semibold text-base-content/70">{{ f?.name ?? '未选择字段' }}</p>
      </div>
      <span v-if="f" class="badge badge-ghost badge-sm ml-auto gap-1.5 border-base-content/10 text-[9px]"><span class="status status-xs" :class="f.enabled ? 'status-success' : 'status-neutral'" />{{ f.enabled ? '启用' : '隐藏' }}</span>
    </header>

    <div v-if="f" class="scroll-thin min-h-0 flex-1 space-y-3 overflow-y-auto p-3">
      <section class="inspector-card">
        <header class="inspector-card-header">
          <span class="inspector-card-icon text-accent"><Database :size="14" /></span>
          <div><h3 class="inspector-card-title">字段内容</h3><p class="inspector-card-caption">名称与数据来源</p></div>
          <span class="badge badge-ghost badge-sm ml-auto border-base-content/10 text-[9px]">{{ kindLabel }}</span>
        </header>
        <div class="inspector-card-body space-y-3">
          <div><label class="label field-label" :for="controlId('name')">字段名称</label><input :id="controlId('name')" v-model="f.name" class="input-compact font-semibold" /></div>
          <div class="grid grid-cols-2 gap-2">
            <div><label class="label field-label" :for="controlId('kind')">数据类型</label><select :id="controlId('kind')" :value="f.kind" class="select-compact" @change="onKindChange"><option value="fixed">固定值</option><option value="csv">CSV 列</option><option value="random">随机值</option><option value="date">日期时间</option></select></div>
            <div v-if="f.kind === 'fixed'"><label class="label field-label" :for="controlId('value')">固定内容</label><input :id="controlId('value')" v-model="f.value" class="input-compact" /></div>
            <div v-else-if="f.kind === 'csv'"><label class="label field-label" :for="controlId('csv-column')">CSV 列名</label><input :id="controlId('csv-column')" v-model="f.csvColumn" class="input-compact" placeholder="column_name" /></div>
          </div>
          <div v-if="f.kind === 'random'" class="inspector-subpanel grid grid-cols-[1fr_1fr_68px] gap-2">
            <div><label class="label field-label" :for="controlId('random-min')">最小值</label><input :id="controlId('random-min')" v-model.number="f.randomMin" type="number" required class="input-compact validator font-mono" /><p class="validator-hint hidden text-[10px]">请输入数值</p></div>
            <div><label class="label field-label" :for="controlId('random-max')">最大值</label><input :id="controlId('random-max')" v-model.number="f.randomMax" type="number" required class="input-compact validator font-mono" /><p class="validator-hint hidden text-[10px]">请输入数值</p></div>
            <div><label class="label field-label" :for="controlId('random-decimals')">小数位</label><input :id="controlId('random-decimals')" v-model.number="f.randomDecimals" type="number" min="0" max="8" required class="input-compact validator font-mono" /><p class="validator-hint hidden text-[10px]">0–8</p></div>
          </div>
          <div v-if="f.kind === 'date'" class="inspector-subpanel space-y-3">
            <div><span class="label field-label">日期时间</span><DateTimeCalendar :key="f.id" v-model:date="f.dateValue" v-model:time="f.timeValue" /></div>
            <div><label class="label field-label" :for="controlId('date-format')">输出格式</label><input :id="controlId('date-format')" v-model="f.dateFormat" class="input-compact font-mono" list="date-format-presets" :placeholder="DEFAULT_DATE_TIME_FORMAT" /></div>
            <datalist id="date-format-presets"><option value="YYYY-MM-DD HH:mm"/><option value="YYYY/MM/DD HH:mm"/><option value="YYYY年MM月DD日 HH:mm"/><option value="DD-MM-YYYY HH:mm"/></datalist>
          </div>
        </div>
      </section>

      <section class="inspector-card">
        <header class="inspector-card-header">
          <span class="inspector-card-icon text-primary"><BoxSelect :size="14" /></span>
          <div><h3 class="inspector-card-title">排版区域</h3><p class="inspector-card-caption">位置与画框尺寸</p></div>
          <span class="ml-auto font-mono text-[9px] text-primary/70">原图 px</span>
        </header>
        <div class="inspector-card-body">
          <div class="grid grid-cols-2 gap-2">
            <div v-for="key in (['x','y','width','height'] as const)" :key="key"><label class="label field-label uppercase" :for="controlId(`layout-${key}`)">{{ key === 'width' ? 'W · 宽度' : key === 'height' ? 'H · 高度' : key }}</label><input :id="controlId(`layout-${key}`)" v-model.number="f.layoutRect[key]" type="number" min="0" required class="input-compact validator font-mono" /><p class="validator-hint hidden text-[10px]">不能小于 0</p></div>
          </div>
          <div class="inspector-subpanel mt-3 space-y-2.5">
            <div class="flex items-center gap-2.5">
              <span class="grid h-7 w-7 place-items-center rounded-md border border-primary/15 bg-primary/10 text-primary"><RotateCw :size="13" /></span>
              <div class="min-w-0 flex-1"><label class="block text-[11px] font-semibold" :for="controlId('rotation-enabled')">旋转画框</label><p class="mt-0.5 text-[9px] text-base-content/40">文字、擦除和补丁使用相同角度</p></div>
              <input :id="controlId('rotation-enabled')" :checked="f.rotationEnabled" type="checkbox" class="toggle toggle-primary toggle-sm" @change="toggleRotation" />
            </div>
            <div v-if="f.rotationEnabled" class="grid grid-cols-[1fr_78px_28px] items-end gap-2 border-t border-base-content/10 pt-2.5">
              <div><label class="label field-label" :for="controlId('rotation-range')">旋转角度</label><input :id="controlId('rotation-range')" :value="f.rotation" type="range" min="-180" max="180" step="1" class="range range-primary range-xs w-full" aria-label="旋转角度滑块" @input="commitRotation" /></div>
              <div><label class="label field-label" :for="controlId('rotation-angle')">角度 °</label><input :id="controlId('rotation-angle')" :value="f.rotation" type="number" min="-180" max="180" step="0.1" class="input-compact validator font-mono" required @change="commitRotation" @blur="commitRotation" /><p class="validator-hint hidden text-[10px]">-180–180</p></div>
              <button type="button" class="btn btn-ghost btn-xs btn-square mb-0.5" title="重置为 0°" aria-label="重置旋转角度" @click="store.setRotation(f.id, 0)"><RotateCcw :size="13" /></button>
            </div>
          </div>
          <details class="collapse collapse-arrow mt-3 rounded-lg border border-base-content/10 bg-base-200/45">
            <summary class="collapse-title min-h-0 px-3 py-2.5 text-[10px] font-medium text-base-content/55">独立调整擦除区域</summary>
            <div class="collapse-content grid grid-cols-2 gap-2 px-3! pb-3!">
              <div v-for="key in (['x','y','width','height'] as const)" :key="key"><label class="label field-label uppercase" :for="controlId(`erase-${key}`)">{{ key === 'width' ? 'W · 宽度' : key === 'height' ? 'H · 高度' : key }}</label><input :id="controlId(`erase-${key}`)" v-model.number="f.eraseRect[key]" type="number" min="0" required class="input-compact validator font-mono" /><p class="validator-hint hidden text-[10px]">不能小于 0</p></div>
            </div>
          </details>
        </div>
      </section>

      <section class="inspector-card">
        <header class="inspector-card-header">
          <span class="inspector-card-icon text-secondary"><TypeIcon :size="15" /></span>
          <div><h3 class="inspector-card-title">文字样式</h3><p class="inspector-card-caption">字体、颜色与对齐</p></div>
        </header>
        <div class="inspector-card-body space-y-3">
          <div class="grid grid-cols-[1fr_76px] gap-2"><div><label class="label field-label" :for="controlId('font-family')">字体</label><select :id="controlId('font-family')" v-model="f.text.fontFamily" class="select-compact"><option>宋体</option><option>仿宋</option><option>黑体</option><option>Arial</option><option>Times New Roman</option></select></div><div><label class="label field-label" :for="controlId('font-size')">字号</label><input :id="controlId('font-size')" v-model.number="f.text.fontSize" type="number" min="1" required class="input-compact validator font-mono" /><p class="validator-hint hidden text-[10px]">至少 1 px</p></div></div>
          <div><label class="label field-label" :for="controlId('text-color')">文字颜色</label><div class="grid grid-cols-[36px_1fr] gap-2"><input :id="controlId('text-color')" v-model="f.text.color" type="color" aria-label="文字颜色" class="h-8 w-9 cursor-pointer rounded-md border border-base-content/10 bg-base-200/60 p-1" /><input v-model="f.text.color" aria-label="文字颜色值" class="input-compact font-mono uppercase" /></div></div>
          <div class="grid grid-cols-2 gap-2">
            <div><label class="label field-label">水平对齐</label><select v-model="f.text.horizontalAlign" aria-label="水平对齐" class="select-compact"><option value="left">左对齐</option><option value="center">居中</option><option value="right">右对齐</option></select></div>
            <div><label class="label field-label">垂直对齐</label><select v-model="f.text.verticalAlign" aria-label="垂直对齐" class="select-compact"><option value="top">顶部</option><option value="middle">居中</option><option value="bottom">底部</option></select></div>
          </div>
        </div>
      </section>

      <section class="inspector-card">
        <header class="inspector-card-header">
          <span class="inspector-card-icon text-warning"><Eraser :size="14" /></span>
          <div><h3 class="inspector-card-title">背景清除</h3><p class="inspector-card-caption">擦除算法与参数</p></div>
          <span class="badge badge-ghost badge-sm ml-auto border-base-content/10 text-[9px]">{{ clearLabel }}</span>
        </header>
        <div class="inspector-card-body">
          <div role="tablist" aria-label="背景清除算法" class="tabs tabs-box tabs-sm grid grid-cols-3 bg-base-200/70 p-1">
            <input v-model="f.clear.mode" type="radio" :name="controlId('clear-mode')" value="inpaint" class="tab h-7 w-full text-[10px]" aria-label="智能抹除" />
            <input v-model="f.clear.mode" type="radio" :name="controlId('clear-mode')" value="telea" class="tab h-7 w-full text-[10px]" aria-label="Telea" />
            <input v-model="f.clear.mode" type="radio" :name="controlId('clear-mode')" value="patch" class="tab h-7 w-full text-[10px]" aria-label="背景补丁" />
          </div>
          <div v-if="f.clear.mode === 'inpaint'" class="inspector-subpanel mt-3 grid grid-cols-2 gap-2">
            <div><label class="label field-label" :for="controlId('inpaint-threshold')">文字阈值</label><input :id="controlId('inpaint-threshold')" v-model.number="f.clear.inpaintThreshold" class="input-compact validator font-mono" type="number" min="1" max="80" required /><p class="validator-hint hidden text-[10px]">范围 1–80</p></div>
            <div><label class="label field-label" :for="controlId('inpaint-radius')">边缘扩张</label><input :id="controlId('inpaint-radius')" v-model.number="f.clear.inpaintRadius" class="input-compact validator font-mono" type="number" min="0" max="6" required /><p class="validator-hint hidden text-[10px]">范围 0–6</p></div>
          </div>
          <div v-else-if="f.clear.mode === 'telea'" class="inspector-subpanel mt-3 grid grid-cols-2 gap-2">
            <div><label class="label field-label" :for="controlId('telea-threshold')">文字阈值</label><input :id="controlId('telea-threshold')" v-model.number="f.clear.teleaThreshold" class="input-compact validator font-mono" type="number" min="1" max="80" required /><p class="validator-hint hidden text-[10px]">范围 1–80</p></div>
            <div><label class="label field-label" :for="controlId('telea-mask-radius')">掩码扩张</label><input :id="controlId('telea-mask-radius')" v-model.number="f.clear.teleaMaskRadius" class="input-compact validator font-mono" type="number" min="0" max="12" required /><p class="validator-hint hidden text-[10px]">范围 0–12</p></div>
            <div><label class="label field-label" :for="controlId('telea-radius')">修复半径</label><input :id="controlId('telea-radius')" v-model.number="f.clear.teleaRadius" class="input-compact validator font-mono" type="number" min="1" max="100" required /><p class="validator-hint hidden text-[10px]">范围 1–100</p></div>
          </div>
          <div v-else class="mt-3"><label class="label field-label" :for="controlId('patch-path')">补丁图片路径</label><input :id="controlId('patch-path')" v-model="f.clear.patchPath" class="input-compact font-mono" placeholder="背景补丁绝对路径" /></div>
          <div class="mt-3 rounded-lg border border-base-content/10 bg-base-200/45 px-3 py-2.5 text-[10px] leading-4 text-base-content/45">
            <p v-if="f.clear.mode === 'inpaint'">阈值越低，识别的深色笔画越多；边缘扩张用于清除抗锯齿残边。请让擦除区域避开表格线。</p>
            <p v-else-if="f.clear.mode === 'telea'">掩码扩张覆盖文字边缘，修复半径控制 Telea 的背景采样范围。</p>
            <p v-else>适合复杂纹理背景。导出时会先覆盖补丁，再绘制新的动态文字。</p>
          </div>
        </div>
      </section>
    </div>

    <div v-else class="grid flex-1 place-items-center px-10 text-center">
      <div><span class="mx-auto mb-3 grid h-10 w-10 place-items-center rounded-xl border border-dashed border-base-content/20 text-base-content/25"><SlidersHorizontal :size="17" /></span><p class="text-xs font-medium text-base-content/45">尚未选择字段</p><p class="mt-1 text-[10px] leading-5 text-base-content/30">在画布或字段图层中选择一个画框<br />即可编辑详细属性</p></div>
    </div>
  </aside>
</template>
