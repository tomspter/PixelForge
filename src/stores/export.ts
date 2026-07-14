import { ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { useFeedbackStore } from './feedback'
import { useTemplateStore } from './template'

export type OutputFormat = 'png' | 'jpg'

export const useExportStore = defineStore('export', () => {
  const template = useTemplateStore()
  const feedback = useFeedbackStore()
  const csvPath = ref('')
  const outputDir = ref('')
  const outputFormat = ref<OutputFormat>('png')
  const busy = ref(false)

  async function chooseCsv() {
    if (!isTauri()) return
    const path = await open({ multiple: false, filters: [{ name: 'CSV 数据', extensions: ['csv'] }] })
    if (!path) return
    csvPath.value = String(path)
    feedback.setNotice('CSV 数据文件已选择', 'success')
  }

  async function chooseOutput() {
    if (!isTauri()) return
    const path = await open({ directory: true, multiple: false })
    if (!path) return
    outputDir.value = String(path)
    feedback.setNotice('输出目录已选择', 'success')
  }

  async function notifyGenerated(count: number) {
    try {
      let granted = await isPermissionGranted()
      if (!granted) granted = await requestPermission() === 'granted'
      if (granted) sendNotification({ title: '像素铸坊 · 生成完成', body: `已成功生成 ${count} 张 ${outputFormat.value.toUpperCase()} 图片` })
    } catch (error) {
      console.warn('发送完成通知失败', error)
    }
  }

  async function generate() {
    if (!template.document || !outputDir.value || !isTauri()) {
      feedback.setNotice('请先选择模板和输出目录', 'warning')
      feedback.showGenerationToast({ type: 'error', title: '无法开始生成', message: feedback.notice })
      return
    }
    busy.value = true
    feedback.setNotice('正在生成图片…', 'working')
    feedback.dismissGenerationToast()
    try {
      const count = await invoke<number>('generate_batch', {
        template: template.document,
        csvPath: csvPath.value || null,
        outputDir: outputDir.value,
        outputFormat: outputFormat.value,
      })
      feedback.setNotice(`已生成 ${count} 张 ${outputFormat.value.toUpperCase()} 图片`, 'success')
      feedback.showGenerationToast({ type: 'success', title: '图片生成完成', message: `已成功生成 ${count} 张 ${outputFormat.value.toUpperCase()} 图片` })
      await notifyGenerated(count)
    } catch (error) {
      const detail = feedback.reportError('图片生成失败', error)
      feedback.showGenerationToast({ type: 'error', title: '图片生成失败', message: detail })
    } finally {
      busy.value = false
    }
  }

  return { csvPath, outputDir, outputFormat, busy, chooseCsv, chooseOutput, generate }
})
