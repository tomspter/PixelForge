import { computed, ref, toRaw, watch } from 'vue'
import { defineStore } from 'pinia'
import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import type { ClearMode, FieldKind, Rect, TemplateDocument, TemplateField } from '../types/template'
import { makeField } from '../types/template'
import { localDateTimeNow } from '../utils/dateTime'
import { useFeedbackStore } from './feedback'

export const useTemplateStore = defineStore('template', () => {
  const feedback = useFeedbackStore()
  const document = ref<TemplateDocument | null>(null)
  const selectedId = ref<string | null>(null)
  const zoom = ref(1)
  const tool = ref<'select' | 'draw'>('select')
  const dirty = ref(false)
  let trackDirty = false
  const selected = computed(() => document.value?.fields.find(f => f.id === selectedId.value) ?? null)
  const backgroundUrl = computed(() => document.value ? (isTauri() ? convertFileSrc(document.value.background.path) : document.value.background.path) : '')

  watch(document, () => {
    if (trackDirty) dirty.value = true
  }, { deep: true, flush: 'sync' })

  function setDocument(next: TemplateDocument) {
    next.fields.forEach(field => {
      if (!['fixed', 'csv', 'random', 'date'].includes(field.kind)) field.kind = 'fixed' as FieldKind
      if (!['inpaint', 'telea', 'patch'].includes(field.clear.mode)) field.clear.mode = 'inpaint' as ClearMode
      // Migrate templates created while both algorithms shared the inpaint settings.
      field.clear.teleaThreshold ??= field.clear.mode === 'telea' ? (field.clear.inpaintThreshold ?? 14) : 14
      field.clear.teleaMaskRadius ??= 1
      field.clear.teleaRadius ??= field.clear.mode === 'telea' ? (field.clear.inpaintRadius ?? 3) : 3
      if (!['宋体', '仿宋', '黑体', 'Arial', 'Times New Roman'].includes(field.text.fontFamily)) field.text.fontFamily = '黑体'
      field.randomMin ??= 0; field.randomMax ??= 100; field.randomDecimals ??= 0
      field.rotationEnabled ??= false
      field.rotation = Number.isFinite(field.rotation) ? normalizeRotation(field.rotation) : 0
      field.dateFormat ||= 'YYYY-MM-DD HH:mm'
      if (field.kind === 'date') {
        const fallback = localDateTimeNow()
        const match = field.value.match(/^(\d{4}-\d{2}-\d{2})(?:[T ](\d{2}:\d{2}))?/)
        field.dateValue ||= match?.[1] ?? fallback.slice(0, 10)
        field.timeValue ||= match?.[2] ?? '00:00'
      }
    })
    trackDirty = false
    document.value = next
    selectedId.value = next.fields[0]?.id ?? null
    dirty.value = false
    trackDirty = true
  }
  function normalizeRotation(value: number) {
    let normalized = value % 360
    if (normalized > 180) normalized -= 360
    if (normalized < -180) normalized += 360
    return Object.is(normalized, -0) ? 0 : Math.round(normalized * 100) / 100
  }
  function fieldHalfExtents(width: number, height: number, rotation: number) {
    const radians = rotation * Math.PI / 180
    const cosine = Math.abs(Math.cos(radians)); const sine = Math.abs(Math.sin(radians))
    return { x: (width * cosine + height * sine) / 2, y: (width * sine + height * cosine) / 2 }
  }
  function clampFieldCenter(field: TemplateField, centerX: number, centerY: number, width = field.layoutRect.width, height = field.layoutRect.height) {
    if (!document.value) return { x: centerX, y: centerY }
    const angle = field.rotationEnabled ? field.rotation : 0
    const half = fieldHalfExtents(width, height, angle)
    const clampAxis = (value: number, extent: number, size: number) => extent * 2 >= size ? size / 2 : Math.min(Math.max(extent, value), size - extent)
    return {
      x: clampAxis(centerX, half.x, document.value.background.width),
      y: clampAxis(centerY, half.y, document.value.background.height),
    }
  }
  function addField(rect: Rect) { if (!document.value) return; const field = makeField(rect, document.value.fields.length + 1); document.value.fields.push(field); selectedId.value = field.id; tool.value = 'select'; dirty.value = true }
  function removeSelected() { if (!document.value || !selectedId.value) return; const i = document.value.fields.findIndex(f => f.id === selectedId.value); document.value.fields.splice(i, 1); selectedId.value = document.value.fields[Math.max(0, i - 1)]?.id ?? null; dirty.value = true }
  function duplicateSelected() {
    if (!document.value || !selected.value) return
    const field = structuredClone(toRaw(selected.value))
    field.id = crypto.randomUUID(); field.name += ' 副本'
    const center = clampFieldCenter(field, field.layoutRect.x + field.layoutRect.width / 2 + 8, field.layoutRect.y + field.layoutRect.height / 2 + 8)
    const nextX = Math.round(center.x - field.layoutRect.width / 2); const nextY = Math.round(center.y - field.layoutRect.height / 2)
    const dx = nextX - field.layoutRect.x; const dy = nextY - field.layoutRect.y
    field.layoutRect.x = nextX; field.layoutRect.y = nextY
    field.eraseRect.x = Math.max(0, field.eraseRect.x + dx); field.eraseRect.y = Math.max(0, field.eraseRect.y + dy)
    document.value.fields.push(field); selectedId.value = field.id; tool.value = 'select'; dirty.value = true
  }
  function nudgeSelected(dx: number, dy: number) {
    if (!document.value || !selected.value) return
    const field = selected.value
    const center = clampFieldCenter(field, field.layoutRect.x + field.layoutRect.width / 2 + dx, field.layoutRect.y + field.layoutRect.height / 2 + dy)
    const nextX = Math.round(center.x - field.layoutRect.width / 2)
    const nextY = Math.round(center.y - field.layoutRect.height / 2)
    const movedX = nextX - field.layoutRect.x; const movedY = nextY - field.layoutRect.y
    if (!movedX && !movedY) return
    field.layoutRect.x = nextX; field.layoutRect.y = nextY
    field.eraseRect.x = Math.max(0, field.eraseRect.x + movedX); field.eraseRect.y = Math.max(0, field.eraseRect.y + movedY)
    dirty.value = true
  }
  function setFieldEnabled(id: string, enabled: boolean) {
    const field = document.value?.fields.find(candidate => candidate.id === id)
    if (field) field.enabled = enabled
  }
  function setFieldKind(id: string, kind: FieldKind) {
    const field = document.value?.fields.find(candidate => candidate.id === id)
    if (!field) return
    field.kind = kind
    if (kind === 'random') {
      field.randomMin ??= 0
      field.randomMax ??= 100
      field.randomDecimals ??= 0
    }
    if (kind === 'date') {
      const now = localDateTimeNow()
      field.dateValue ||= now.slice(0, 10)
      field.timeValue ||= now.slice(11, 16)
      field.dateFormat ||= 'YYYY-MM-DD HH:mm'
    }
  }
  function reorderFields(next: TemplateField[]) {
    if (!document.value || next.length !== document.value.fields.length) return
    const currentIds = new Set(document.value.fields.map(field => field.id))
    if (new Set(next.map(field => field.id)).size !== currentIds.size || next.some(field => !currentIds.has(field.id))) return
    document.value.fields = [...next]
    dirty.value = true
  }
  function scaleLinkedRect(linked: Rect, previous: Rect, next: Rect): Rect {
    const scaleX = previous.width > 0 ? next.width / previous.width : 1
    const scaleY = previous.height > 0 ? next.height / previous.height : 1
    return {
      x: Math.max(0, Math.round(next.x + (linked.x - previous.x) * scaleX)),
      y: Math.max(0, Math.round(next.y + (linked.y - previous.y) * scaleY)),
      width: Math.max(0, Math.round(linked.width * scaleX)),
      height: Math.max(0, Math.round(linked.height * scaleY)),
    }
  }
  function updateFieldRect(id: string, next: Rect, syncErase = false) {
    const field = document.value?.fields.find(candidate => candidate.id === id)
    if (!field) return
    if (syncErase) field.eraseRect = scaleLinkedRect(field.eraseRect, field.layoutRect, next)
    field.layoutRect = next
  }
  function updateFieldTransform(id: string, next: Rect, rotation: number, syncErase = false) {
    const field = document.value?.fields.find(candidate => candidate.id === id)
    if (!field) return
    if (field.rotationEnabled) field.rotation = normalizeRotation(rotation)
    const center = clampFieldCenter(field, next.x + next.width / 2, next.y + next.height / 2, next.width, next.height)
    const clamped = { ...next, x: Math.round(center.x - next.width / 2), y: Math.round(center.y - next.height / 2) }
    updateFieldRect(id, clamped, syncErase)
  }
  function setRotation(id: string, value: number) {
    const field = document.value?.fields.find(candidate => candidate.id === id)
    if (!field) return
    field.rotation = normalizeRotation(Number.isFinite(value) ? value : 0)
    const center = clampFieldCenter(field, field.layoutRect.x + field.layoutRect.width / 2, field.layoutRect.y + field.layoutRect.height / 2)
    const dx = Math.round(center.x - field.layoutRect.width / 2) - field.layoutRect.x
    const dy = Math.round(center.y - field.layoutRect.height / 2) - field.layoutRect.y
    field.layoutRect.x += dx; field.layoutRect.y += dy
    field.eraseRect.x = Math.max(0, field.eraseRect.x + dx); field.eraseRect.y = Math.max(0, field.eraseRect.y + dy)
  }
  function setRotationEnabled(id: string, enabled: boolean) {
    const field = document.value?.fields.find(candidate => candidate.id === id)
    if (!field) return
    field.rotationEnabled = enabled
    if (enabled) setRotation(id, field.rotation)
  }
  async function importBackground() {
    if (!isTauri()) { feedback.setNotice('请在桌面应用中导入本地图片', 'warning'); return }
    const path = await open({ multiple: false, filters: [{ name: '图片文件', extensions: ['png', 'jpg', 'jpeg'] }] })
    if (!path) return
    feedback.setNotice('正在读取图片…', 'working')
    try {
      const meta = await invoke<{ width: number; height: number }>('inspect_image', { path })
      const now = new Date().toISOString()
      setDocument({ schemaVersion: 1, id: crypto.randomUUID(), name: String(path).split(/[\\/]/).pop()?.replace(/\.(png|jpe?g)$/i, '') ?? '未命名模板', background: { path: String(path), ...meta }, fields: [], createdAt: now, updatedAt: now })
      feedback.setNotice(`已载入 ${meta.width} × ${meta.height} 图片`, 'success')
    } catch (error) { feedback.reportError('图片导入失败', error) }
  }
  async function openTemplate() {
    if (!isTauri()) return
    const path = await open({ multiple: false, filters: [{ name: 'PNG 模板', extensions: ['json'] }] })
    if (!path) return
    feedback.setNotice('正在打开模板…', 'working')
    try {
      setDocument(await invoke('load_template', { path }))
      feedback.setNotice('模板已打开', 'success')
    } catch (error) { feedback.reportError('模板打开失败', error) }
  }
  async function saveTemplate() {
    if (!document.value || !isTauri()) return
    const path = await save({ defaultPath: `${document.value.name}.pngtpl.json`, filters: [{ name: 'PNG 模板', extensions: ['json'] }] })
    if (!path) return
    feedback.setNotice('正在保存模板…', 'working')
    try {
      document.value.updatedAt = new Date().toISOString()
      await invoke('save_template', { path, template: document.value })
      dirty.value = false; feedback.setNotice('模板已保存', 'success')
    } catch (error) { feedback.reportError('模板保存失败', error) }
  }
  return { document, selectedId, selected, backgroundUrl, zoom, tool, dirty, setDocument, addField, removeSelected, duplicateSelected, nudgeSelected, setFieldEnabled, setFieldKind, reorderFields, updateFieldRect, updateFieldTransform, setRotation, setRotationEnabled, importBackground, openTemplate, saveTemplate }
})
