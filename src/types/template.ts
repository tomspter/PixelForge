export type FieldKind = 'fixed' | 'csv' | 'random' | 'date'
export type ClearMode = 'inpaint' | 'patch'
export type HAlign = 'left' | 'center' | 'right'
export type VAlign = 'top' | 'middle' | 'bottom'

export interface Rect { x: number; y: number; width: number; height: number }
export interface TextStyle {
  fontFamily: string
  fontSize: number
  color: string
  horizontalAlign: HAlign
  verticalAlign: VAlign
  lineHeight: number
  letterSpacing: number
  padding: number
}
export interface ClearStrategy {
  mode: ClearMode
  patchPath?: string
  inpaintThreshold?: number
  inpaintRadius?: number
}
export interface TemplateField {
  id: string
  name: string
  kind: FieldKind
  value: string
  csvColumn?: string
  dateValue?: string
  timeValue?: string
  dateFormat?: string
  randomMin?: number
  randomMax?: number
  randomDecimals?: number
  eraseRect: Rect
  layoutRect: Rect
  text: TextStyle
  clear: ClearStrategy
  enabled: boolean
}
export interface TemplateDocument {
  schemaVersion: 1
  id: string
  name: string
  background: { path: string; width: number; height: number; sha256?: string }
  fields: TemplateField[]
  createdAt: string
  updatedAt: string
}

export const makeField = (rect: Rect, index: number): TemplateField => ({
  id: crypto.randomUUID(), name: `字段 ${index}`, kind: 'fixed', value: '示例文本', dateValue: '', timeValue: '', dateFormat: 'YYYY-MM-DD HH:mm',
  eraseRect: { ...rect }, layoutRect: { ...rect },
  text: { fontFamily: 'Arial', fontSize: Math.max(12, Math.round(rect.height * .56)), color: '#161916', horizontalAlign: 'center', verticalAlign: 'middle', lineHeight: 1.2, letterSpacing: 0, padding: 4 },
  clear: { mode: 'inpaint', inpaintThreshold: 5, inpaintRadius: 5 }, enabled: true,
})
