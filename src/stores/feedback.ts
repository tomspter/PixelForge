import { ref } from 'vue'
import { defineStore } from 'pinia'

export type NoticeType = 'idle' | 'working' | 'success' | 'warning' | 'error'
export type GenerationToast = { type: 'success' | 'error'; title: string; message: string }

export const useFeedbackStore = defineStore('feedback', () => {
  const notice = ref('就绪')
  const noticeType = ref<NoticeType>('idle')
  const generationToast = ref<GenerationToast | null>(null)
  let toastTimer: ReturnType<typeof setTimeout> | undefined

  function setNotice(message: string, type: NoticeType = 'idle') {
    notice.value = message
    noticeType.value = type
  }

  function readableError(error: unknown) {
    const message = (error instanceof Error ? error.message : String(error))
      .replace(/^(?:Error|InvokeError):\s*/i, '')
      .replace(/No such file or directory(?: \(os error 2\))?/gi, '文件或文件夹不存在')
      .replace(/Permission denied(?: \(os error 13\))?/gi, '没有访问权限')
      .replace(/The system cannot find the file specified\.?(?: \(os error 2\))?/gi, '文件或文件夹不存在')
      .replace(/There is not enough space on the disk\.?(?: \(os error 112\))?/gi, '磁盘空间不足')
      .trim()
    return message || '发生未知错误，请重试'
  }

  function reportError(action: string, error: unknown) {
    const detail = readableError(error)
    setNotice(detail.startsWith(action) || detail.includes('失败') ? detail : `${action}：${detail}`, 'error')
    return detail
  }

  function dismissGenerationToast() {
    clearTimeout(toastTimer)
    generationToast.value = null
  }

  function showGenerationToast(toast: GenerationToast) {
    clearTimeout(toastTimer)
    generationToast.value = toast
    toastTimer = setTimeout(() => generationToast.value = null, toast.type === 'success' ? 5000 : 8000)
  }

  return { notice, noticeType, generationToast, setNotice, readableError, reportError, dismissGenerationToast, showGenerationToast }
})
