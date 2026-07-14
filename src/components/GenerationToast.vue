<script setup lang="ts">
import { CheckCircle2, X, XCircle } from '@lucide/vue'
import { useFeedbackStore } from '../stores/feedback'
const feedback = useFeedbackStore()
</script>

<template>
  <Transition enter-active-class="transition duration-200 ease-out" enter-from-class="translate-x-4 opacity-0" leave-active-class="transition duration-150 ease-in" leave-to-class="translate-x-4 opacity-0">
    <div v-if="feedback.generationToast" class="toast toast-end toast-top z-[80] mt-14">
      <div role="alert" class="alert w-[360px] grid-cols-[auto_1fr_auto] border shadow-2xl" :class="feedback.generationToast.type === 'success' ? 'alert-success border-success/30' : 'alert-error border-error/30'">
        <CheckCircle2 v-if="feedback.generationToast.type === 'success'" :size="20" />
        <XCircle v-else :size="20" />
        <div class="min-w-0"><h3 class="text-sm font-bold">{{ feedback.generationToast.title }}</h3><p class="mt-0.5 break-words text-xs opacity-75">{{ feedback.generationToast.message }}</p></div>
        <button class="tooltip tooltip-left btn btn-ghost btn-xs btn-circle inline-flex!" data-tip="关闭消息" aria-label="关闭消息" @click="feedback.dismissGenerationToast"><X :size="14" /></button>
      </div>
    </div>
  </Transition>
</template>
