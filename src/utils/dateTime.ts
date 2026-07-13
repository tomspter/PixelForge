export const DEFAULT_DATE_TIME_FORMAT = 'YYYY-MM-DD HH:mm'

export function formatDateTime(value: string, pattern = DEFAULT_DATE_TIME_FORMAT) {
  const match = value.match(/^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})/)
  if (!match) return value
  const [, YYYY, MM, DD, HH, mm] = match
  const parts: Record<string, string> = { YYYY, MM, DD, HH, mm }
  return (pattern || DEFAULT_DATE_TIME_FORMAT).replace(/YYYY|MM|DD|HH|mm/g, token => parts[token])
}

export function localDateTimeNow() {
  const now = new Date()
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}T${pad(now.getHours())}:${pad(now.getMinutes())}`
}
