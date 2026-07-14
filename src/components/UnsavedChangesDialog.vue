<script setup lang="ts">
import { ref } from 'vue'
import { AlertTriangle, Save, Trash2 } from '@lucide/vue'
import { useTemplateStore } from '../stores/template'
import { useUnsavedChangesStore } from '../stores/unsavedChanges'

const template = useTemplateStore()
const unsaved = useUnsavedChangesStore()
const saving = ref(false)

function cancel() {
  if (!saving.value) unsaved.respond('cancel')
}

function discard() {
  if (!saving.value) unsaved.respond('discard')
}

async function saveAndContinue() {
  if (saving.value) return
  saving.value = true
  const saved = await template.saveTemplate()
  saving.value = false
  if (saved) unsaved.respond('save')
}
</script>

<template>
  <dialog
    v-if="unsaved.request"
    open
    class="modal modal-open z-[90]"
    aria-labelledby="unsaved-dialog-title"
    aria-describedby="unsaved-dialog-description"
    @cancel.prevent="cancel"
  >
    <div class="modal-box w-[30rem] max-w-[calc(100vw-2rem)] overflow-hidden border border-base-content/12 bg-base-100 p-0 shadow-2xl">
      <div class="flex items-start gap-3.5 border-b border-base-content/10 px-5 py-5">
        <span class="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-warning/25 bg-warning/12 text-warning">
          <AlertTriangle :size="19" stroke-width="2" />
        </span>
        <div class="min-w-0 pt-0.5">
          <p class="mb-1 text-[9px] font-bold uppercase tracking-[.2em] text-warning/80">未保存更改</p>
          <h2 id="unsaved-dialog-title" class="text-sm font-black tracking-tight text-base-content">{{ unsaved.request.title }}</h2>
          <p id="unsaved-dialog-description" class="mt-2 text-[11px] leading-5 text-base-content/55">{{ unsaved.request.message }}</p>
        </div>
      </div>

      <div class="flex items-center gap-2 bg-base-200/55 px-5 py-4">
        <button type="button" class="btn btn-ghost btn-sm mr-auto min-w-16" :disabled="saving" @click="cancel">取消</button>
        <button type="button" class="btn btn-ghost btn-sm text-error hover:bg-error/10" :disabled="saving" @click="discard">
          <Trash2 :size="14" />不保存
        </button>
        <button type="button" class="btn btn-primary btn-sm min-w-32 px-4" :disabled="saving" autofocus @click="saveAndContinue">
          <span v-if="saving" class="loading loading-spinner loading-xs" />
          <Save v-else :size="14" />
          {{ saving ? '正在保存…' : `保存并${unsaved.request.continueLabel}` }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop bg-neutral/45 backdrop-blur-[2px]">
      <button aria-label="取消当前操作" :disabled="saving" @click.prevent="cancel">cancel</button>
    </form>
  </dialog>
</template>
