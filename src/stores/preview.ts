import { ref, watch } from 'vue'
import { defineStore } from 'pinia'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { useExportStore } from './export'
import { useFeedbackStore } from './feedback'
import { useTemplateStore } from './template'

export const usePreviewStore = defineStore('preview', () => {
  const template = useTemplateStore()
  const exportStore = useExportStore()
  const feedback = useFeedbackStore()
  const previewUrl = ref('')
  const previewOpen = ref(false)
  const previewBusy = ref(false)
  const previewZoom = ref(1)

  function disposePreview() {
    if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
    previewUrl.value = ''
  }

  function closePreview() {
    previewOpen.value = false
  }

  async function createPreview() {
    if (!template.document || !isTauri()) {
      feedback.setNotice('请先在桌面应用中打开模板', 'warning')
      return
    }
    if (!previewOpen.value) previewZoom.value = template.zoom
    previewOpen.value = true
    previewBusy.value = true
    feedback.setNotice('正在渲染最终预览…', 'working')
    try {
      const bytes = await invoke<number[]>('render_preview', {
        template: template.document,
        csvPath: exportStore.csvPath || null,
        outputFormat: exportStore.outputFormat,
      })
      disposePreview()
      previewUrl.value = URL.createObjectURL(new Blob([Uint8Array.from(bytes).buffer], { type: exportStore.outputFormat === 'png' ? 'image/png' : 'image/jpeg' }))
      feedback.setNotice(exportStore.csvPath ? '预览已使用 CSV 第一行数据' : '最终效果预览已更新', 'success')
    } catch (error) {
      feedback.reportError('预览生成失败', error)
    } finally {
      previewBusy.value = false
    }
  }

  watch(() => template.document?.id, () => {
    closePreview()
    disposePreview()
  })

  return { previewUrl, previewOpen, previewBusy, previewZoom, createPreview, closePreview, disposePreview }
})
