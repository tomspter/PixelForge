<script setup lang="ts">
import { computed } from 'vue'
import { useTemplateStore } from '../stores/template'
import { DEFAULT_DATE_TIME_FORMAT, localDateTimeNow } from '../utils/dateTime'
const store = useTemplateStore()
const f = computed(() => store.selected)
function onKindChange() {
  if (!f.value) return
  if (f.value.kind === 'random') { f.value.randomMin ??= 0; f.value.randomMax ??= 100; f.value.randomDecimals ??= 0 }
  if (f.value.kind === 'date') {
    const now = localDateTimeNow()
    f.value.dateValue ||= now.slice(0, 10)
    f.value.timeValue ||= now.slice(11, 16)
    f.value.dateFormat ||= DEFAULT_DATE_TIME_FORMAT
  }
}
</script>
<template>
  <aside class="scroll-thin w-72 shrink-0 overflow-y-auto border-l border-base-300 bg-base-200">
    <div class="sticky top-0 z-10 flex h-12 items-center border-b border-base-300 bg-base-200 px-4"><span class="panel-title">属性检查器</span></div>
    <div v-if="f" class="space-y-5 p-4" @change="store.markDirty">
      <section>
        <label class="field-label">字段名称</label><input v-model="f.name" class="input-compact font-semibold" />
        <div class="mt-3 grid grid-cols-2 gap-2">
          <div><label class="field-label">数据类型</label><select v-model="f.kind" class="select-compact" @change="onKindChange"><option value="fixed">固定值</option><option value="csv">CSV 列</option><option value="random">随机值</option><option value="date">日期时间</option></select></div>
          <div v-if="f.kind === 'fixed'"><label class="field-label">固定内容</label><input v-model="f.value" class="input-compact" /></div>
          <div v-else-if="f.kind === 'csv'"><label class="field-label">CSV 列名</label><input v-model="f.csvColumn" class="input-compact" placeholder="column_name" /></div>
        </div>
        <div v-if="f.kind === 'random'" class="mt-2 grid grid-cols-[1fr_1fr_72px] gap-2 rounded-md border border-base-300 bg-base-100 p-2">
          <div><label class="field-label">最小值</label><input v-model.number="f.randomMin" type="number" class="input-compact font-mono" /></div>
          <div><label class="field-label">最大值</label><input v-model.number="f.randomMax" type="number" class="input-compact font-mono" /></div>
          <div><label class="field-label">小数位</label><input v-model.number="f.randomDecimals" type="number" min="0" max="8" class="input-compact font-mono" /></div>
        </div>
        <div v-if="f.kind === 'date'" class="mt-2 grid grid-cols-2 gap-2 rounded-md border border-base-300 bg-base-100 p-2">
          <div><label class="field-label">日期</label><input v-model="f.dateValue" type="date" class="input-compact font-mono" /></div>
          <div><label class="field-label">时间</label><input v-model="f.timeValue" type="time" step="60" class="input-compact font-mono" /></div>
          <div class="col-span-2"><label class="field-label">输出格式</label><input v-model="f.dateFormat" class="input-compact font-mono" list="date-format-presets" :placeholder="DEFAULT_DATE_TIME_FORMAT" /></div>
          <datalist id="date-format-presets"><option value="YYYY-MM-DD HH:mm"/><option value="YYYY/MM/DD HH:mm"/><option value="YYYY年MM月DD日 HH:mm"/><option value="DD-MM-YYYY HH:mm"/></datalist>
          <!-- <p class="col-span-2 text-[10px] text-base-content/35">可用标记：YYYY 年、MM 月、DD 日、HH 时、mm 分</p> -->
        </div>
      </section>
      <section class="border-t border-base-300 pt-4">
        <div class="mb-3 flex items-center justify-between"><span class="panel-title">排版区域</span><span class="font-mono text-[9px] text-accent">原图 px</span></div>
        <div class="grid grid-cols-4 gap-1.5">
          <div v-for="key in (['x','y','width','height'] as const)" :key="key"><label class="field-label uppercase">{{ key === 'width' ? 'W' : key === 'height' ? 'H' : key }}</label><input v-model.number="f.layoutRect[key]" type="number" min="0" class="input-compact px-2 font-mono" /></div>
        </div>
        <details class="mt-2 rounded-md border border-base-300 bg-base-100 px-2 py-1.5">
          <summary class="cursor-pointer text-[10px] text-base-content/45">独立调整擦除区域</summary>
          <div class="mt-2 grid grid-cols-4 gap-1.5">
            <div v-for="key in (['x','y','width','height'] as const)" :key="key"><label class="field-label uppercase">{{ key === 'width' ? 'W' : key === 'height' ? 'H' : key }}</label><input v-model.number="f.eraseRect[key]" type="number" min="0" class="input-compact px-2 font-mono" /></div>
          </div>
        </details>
      </section>
      <section class="border-t border-base-300 pt-4">
        <span class="panel-title mb-3 block">文字样式</span>
        <div class="grid grid-cols-[1fr_72px] gap-2"><div><label class="field-label">字体</label><select v-model="f.text.fontFamily" class="select-compact"><option>宋体</option><option>仿宋</option><option>黑体</option><option>Arial</option><option>Times New Roman</option></select></div><div><label class="field-label">字号</label><input v-model.number="f.text.fontSize" type="number" class="input-compact font-mono" /></div></div>
        <div class="mt-3 grid grid-cols-[44px_1fr] gap-2"><input v-model="f.text.color" type="color" class="h-8 w-11 cursor-pointer rounded border border-base-300 bg-transparent p-1" /><input v-model="f.text.color" class="input-compact font-mono uppercase" /></div>
        <div class="mt-3 grid grid-cols-2 gap-2">
          <select v-model="f.text.horizontalAlign" class="select-compact"><option value="left">水平 · 左</option><option value="center">水平 · 居中</option><option value="right">水平 · 右</option></select>
          <select v-model="f.text.verticalAlign" class="select-compact"><option value="top">垂直 · 顶部</option><option value="middle">垂直 · 居中</option><option value="bottom">垂直 · 底部</option></select>
        </div>
      </section>
      <section class="border-t border-base-300 pt-4">
        <span class="panel-title mb-3 block">背景清除</span>
        <select v-model="f.clear.mode" class="select-compact"><option value="inpaint">智能文字抹除 · 推荐</option><option value="patch">背景补丁</option></select>
        <div v-if="f.clear.mode === 'inpaint'" class="mt-2 grid grid-cols-2 gap-2">
          <div><label class="field-label">文字阈值</label><input v-model.number="f.clear.inpaintThreshold" class="input-compact font-mono" type="number" min="1" max="80" /></div>
          <div><label class="field-label">边缘扩张</label><input v-model.number="f.clear.inpaintRadius" class="input-compact font-mono" type="number" min="0" max="6" /></div>
        </div>
        <input v-if="f.clear.mode === 'patch'" v-model="f.clear.patchPath" class="input-compact mt-2 font-mono" placeholder="背景补丁绝对路径" />
        <p v-if="f.clear.mode === 'inpaint'" class="mt-2 text-[10px] leading-4 text-base-content/35">仅修改比局部背景明显更暗的笔画像素。阈值越低识别越多，边缘扩张用于清除抗锯齿残边；请让画框避开表格线。</p>
        <p v-else class="mt-2 text-[10px] leading-4 text-base-content/35">导出时先执行遮罩，再绘制动态文字。复杂纹理可使用与模板绑定的背景补丁。</p>
      </section>
    </div>
    <div v-else class="grid h-64 place-items-center px-10 text-center text-xs leading-6 text-base-content/30">选择一个画框<br />即可编辑字段属性</div>
  </aside>
</template>
