<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { Calendar, type Options } from 'vanilla-calendar-pro'
import { localDateTimeNow } from '../utils/dateTime'

const props = defineProps<{
  date?: string
  time?: string
}>()

const emit = defineEmits<{
  'update:date': [value: string]
  'update:time': [value: string]
  change: []
}>()

const calendarElement = ref<HTMLElement | null>(null)
let calendar: Calendar | null = null

function initialDate() {
  return /^\d{4}-\d{2}-\d{2}$/.test(props.date ?? '')
    ? props.date!
    : localDateTimeNow().slice(0, 10)
}

function initialTime() {
  return /^([01]\d|2[0-3]):[0-5]\d$/.test(props.time ?? '')
    ? props.time!
    : localDateTimeNow().slice(11, 16)
}

function selectDate(self: Calendar) {
  const value = self.context.selectedDates[0]
  if (!value || value === props.date) return
  emit('update:date', value)
  emit('change')
}

function selectTime(self: Calendar, isError: boolean) {
  const value = self.context.selectedTime.slice(0, 5)
  if (isError || !/^([01]\d|2[0-3]):[0-5]\d$/.test(value) || value === props.time) return
  emit('update:time', value)
  emit('change')
}

onMounted(() => {
  if (!calendarElement.value) return

  const options: Options = {
    locale: 'zh-CN',
    firstWeekday: 1,
    enableDateToggle: false,
    selectionDatesMode: 'single',
    selectionTimeMode: 24,
    selectedDates: [initialDate()],
    selectedTime: initialTime(),
    timeControls: 'all',
    timeStepMinute: 1,
    onClickDate: selectDate,
    onChangeTime(self, _event, isError) {
      selectTime(self, isError)
    },
  }

  calendar = new Calendar(calendarElement.value, options)
  calendar.init()
})

onBeforeUnmount(() => {
  calendar?.destroy()
  calendar = null
})
</script>

<template>
  <div ref="calendarElement" class="vc" aria-label="日期和时间选择器" />
</template>
