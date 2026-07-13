import { computed, ref, toRaw } from 'vue'
import { defineStore } from 'pinia'
import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import type { ClearMode, FieldKind, Rect, TemplateDocument } from '../types/template'
import { makeField } from '../types/template'
import { localDateTimeNow } from '../utils/dateTime'

export const useTemplateStore = defineStore('template', () => {
  type GenerationToast = { type: 'success' | 'error'; title: string; message: string }
  const document = ref<TemplateDocument | null>(null)
  const selectedId = ref<string | null>(null)
  const zoom = ref(1)
  const tool = ref<'select' | 'draw'>('select')
  const dirty = ref(false)
  const csvPath = ref('')
  const outputDir = ref('')
  const outputFormat = ref<'png' | 'jpg'>('png')
  const previewUrl = ref('')
  const previewOpen = ref(false)
  const previewBusy = ref(false)
  const previewZoom = ref(1)
  const busy = ref(false)
  const notice = ref('就绪')
  const generationToast = ref<GenerationToast | null>(null)
  let toastTimer: ReturnType<typeof setTimeout> | undefined
  const selected = computed(() => document.value?.fields.find(f => f.id === selectedId.value) ?? null)
  const backgroundUrl = computed(() => document.value ? (isTauri() ? convertFileSrc(document.value.background.path) : document.value.background.path) : '')

  function setDocument(next: TemplateDocument) {
    next.fields.forEach(field => {
      if (!['fixed', 'csv', 'random', 'date'].includes(field.kind)) field.kind = 'fixed' as FieldKind
      if (!['inpaint', 'patch'].includes(field.clear.mode)) field.clear.mode = 'inpaint' as ClearMode
      if (!['宋体', '仿宋', '黑体', 'Arial', 'Times New Roman'].includes(field.text.fontFamily)) field.text.fontFamily = '黑体'
      field.randomMin ??= 0; field.randomMax ??= 100; field.randomDecimals ??= 0
      field.dateFormat ||= 'YYYY-MM-DD HH:mm'
      if (field.kind === 'date') {
        const fallback = localDateTimeNow()
        const match = field.value.match(/^(\d{4}-\d{2}-\d{2})(?:[T ](\d{2}:\d{2}))?/)
        field.dateValue ||= match?.[1] ?? fallback.slice(0, 10)
        field.timeValue ||= match?.[2] ?? '00:00'
      }
    })
    document.value = next; selectedId.value = next.fields[0]?.id ?? null; dirty.value = false
  }
  function addField(rect: Rect) { if (!document.value) return; const field = makeField(rect, document.value.fields.length + 1); document.value.fields.push(field); selectedId.value = field.id; tool.value = 'select'; dirty.value = true }
  function removeSelected() { if (!document.value || !selectedId.value) return; const i = document.value.fields.findIndex(f => f.id === selectedId.value); document.value.fields.splice(i, 1); selectedId.value = document.value.fields[Math.max(0, i - 1)]?.id ?? null; dirty.value = true }
  function duplicateSelected() {
    if (!document.value || !selected.value) return
    const field = structuredClone(toRaw(selected.value))
    field.id = crypto.randomUUID(); field.name += ' 副本'
    const maxX = Math.max(0, document.value.background.width - field.layoutRect.width)
    const maxY = Math.max(0, document.value.background.height - field.layoutRect.height)
    const nextX = Math.min(maxX, field.layoutRect.x + 8); const nextY = Math.min(maxY, field.layoutRect.y + 8)
    const dx = nextX - field.layoutRect.x; const dy = nextY - field.layoutRect.y
    field.layoutRect.x = nextX; field.layoutRect.y = nextY
    field.eraseRect.x = Math.max(0, field.eraseRect.x + dx); field.eraseRect.y = Math.max(0, field.eraseRect.y + dy)
    document.value.fields.push(field); selectedId.value = field.id; tool.value = 'select'; dirty.value = true
  }
  function nudgeSelected(dx: number, dy: number) {
    if (!document.value || !selected.value) return
    const field = selected.value
    const nextX = Math.min(Math.max(0, field.layoutRect.x + dx), Math.max(0, document.value.background.width - field.layoutRect.width))
    const nextY = Math.min(Math.max(0, field.layoutRect.y + dy), Math.max(0, document.value.background.height - field.layoutRect.height))
    const movedX = nextX - field.layoutRect.x; const movedY = nextY - field.layoutRect.y
    if (!movedX && !movedY) return
    field.layoutRect.x = nextX; field.layoutRect.y = nextY
    field.eraseRect.x = Math.max(0, field.eraseRect.x + movedX); field.eraseRect.y = Math.max(0, field.eraseRect.y + movedY)
    dirty.value = true
  }
  function markDirty() { dirty.value = true }
  function closePreview() { previewOpen.value = false }
  function dismissGenerationToast() { clearTimeout(toastTimer); generationToast.value = null }
  function showGenerationToast(toast: GenerationToast) {
    clearTimeout(toastTimer); generationToast.value = toast
    toastTimer = setTimeout(() => generationToast.value = null, toast.type === 'success' ? 5000 : 8000)
  }

  async function createPreview() {
    if (!document.value || !isTauri()) { notice.value = '请先在桌面应用中打开模板'; return }
    if (!previewOpen.value) previewZoom.value = zoom.value
    previewOpen.value = true; previewBusy.value = true; notice.value = '正在渲染最终预览…'
    try {
      const bytes = await invoke<number[]>('render_preview', { template: document.value, csvPath: csvPath.value || null, outputFormat: outputFormat.value })
      if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
      previewUrl.value = URL.createObjectURL(new Blob([Uint8Array.from(bytes).buffer], { type: outputFormat.value === 'png' ? 'image/png' : 'image/jpeg' }))
      notice.value = csvPath.value ? '预览已使用 CSV 第一行数据' : '最终效果预览已更新'
    } catch (error) { notice.value = `预览失败：${String(error)}` }
    finally { previewBusy.value = false }
  }

  async function notifyGenerated(count: number) {
    try {
      let granted = await isPermissionGranted()
      if (!granted) granted = await requestPermission() === 'granted'
      if (granted) sendNotification({ title: '像素铸坊 · 生成完成', body: `已成功生成 ${count} 张 ${outputFormat.value.toUpperCase()} 图片` })
    } catch (error) { console.warn('发送完成通知失败', error) }
  }

  async function importBackground() {
    if (!isTauri()) { notice.value = '浏览器预览模式下请通过 Tauri 打开本地图片'; return }
    const path = await open({ multiple: false, filters: [{ name: '图片文件', extensions: ['png', 'jpg', 'jpeg'] }] })
    if (!path) return
    const meta = await invoke<{ width: number; height: number }>('inspect_image', { path })
    const now = new Date().toISOString()
    setDocument({ schemaVersion: 1, id: crypto.randomUUID(), name: String(path).split(/[\\/]/).pop()?.replace(/\.(png|jpe?g)$/i, '') ?? '未命名模板', background: { path: String(path), ...meta }, fields: [], createdAt: now, updatedAt: now })
    notice.value = `已载入 ${meta.width} × ${meta.height} 图片`
  }
  async function openTemplate() {
    if (!isTauri()) return
    const path = await open({ multiple: false, filters: [{ name: 'PNG 模板', extensions: ['json'] }] })
    if (path) setDocument(await invoke('load_template', { path }))
  }
  async function saveTemplate() {
    if (!document.value || !isTauri()) return
    const path = await save({ defaultPath: `${document.value.name}.pngtpl.json`, filters: [{ name: 'PNG 模板', extensions: ['json'] }] })
    if (!path) return
    document.value.updatedAt = new Date().toISOString()
    await invoke('save_template', { path, template: document.value })
    dirty.value = false; notice.value = '模板已保存'
  }
  async function chooseCsv() { if (!isTauri()) return; const p = await open({ multiple: false, filters: [{ name: 'CSV 数据', extensions: ['csv'] }] }); if (p) csvPath.value = String(p) }
  async function chooseOutput() { if (!isTauri()) return; const p = await open({ directory: true, multiple: false }); if (p) outputDir.value = String(p) }
  async function generate() {
    if (!document.value || !outputDir.value || !isTauri()) {
      notice.value = '请先选择模板和输出目录'
      showGenerationToast({ type: 'error', title: '无法开始生成', message: notice.value })
      return
    }
    busy.value = true; notice.value = '正在生成…'
    dismissGenerationToast()
    try {
      const count = await invoke<number>('generate_batch', { template: document.value, csvPath: csvPath.value || null, outputDir: outputDir.value, outputFormat: outputFormat.value })
      notice.value = `完成：已生成 ${count} 张 ${outputFormat.value.toUpperCase()} 图片`
      showGenerationToast({ type: 'success', title: '图片生成完成', message: `已成功生成 ${count} 张 ${outputFormat.value.toUpperCase()} 图片` })
      await notifyGenerated(count)
    }
    catch (e) {
      notice.value = `生成失败：${String(e)}`
      showGenerationToast({ type: 'error', title: '图片生成失败', message: String(e) })
    }
    finally { busy.value = false }
  }
  return { document, selectedId, selected, backgroundUrl, zoom, tool, dirty, csvPath, outputDir, outputFormat, previewUrl, previewOpen, previewBusy, previewZoom, generationToast, busy, notice, setDocument, addField, removeSelected, duplicateSelected, nudgeSelected, markDirty, closePreview, dismissGenerationToast, createPreview, importBackground, openTemplate, saveTemplate, chooseCsv, chooseOutput, generate }
})
