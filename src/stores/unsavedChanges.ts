import { ref } from 'vue'
import { defineStore } from 'pinia'

export type UnsavedChoice = 'save' | 'discard' | 'cancel'

export interface UnsavedRequest {
  title: string
  message: string
  continueLabel: string
}

export const useUnsavedChangesStore = defineStore('unsaved-changes', () => {
  const request = ref<UnsavedRequest | null>(null)
  let resolveRequest: ((choice: UnsavedChoice) => void) | null = null

  function confirm(next: UnsavedRequest): Promise<UnsavedChoice> {
    if (request.value) return Promise.resolve('cancel')
    request.value = next
    return new Promise(resolve => { resolveRequest = resolve })
  }

  function respond(choice: UnsavedChoice) {
    const resolve = resolveRequest
    resolveRequest = null
    request.value = null
    resolve?.(choice)
  }

  return { request, confirm, respond }
})
