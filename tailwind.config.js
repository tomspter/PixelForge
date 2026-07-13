import daisyui from 'daisyui'
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  theme: { extend: { fontFamily: { sans: ['Avenir Next', 'Noto Sans SC', 'sans-serif'], mono: ['JetBrains Mono', 'monospace'] } } },
  daisyui: { themes: [{ forge: { primary: '#d8ff3e', secondary: '#ff7657', accent: '#59d8ff', neutral: '#1b1e1b', 'base-100': '#111311', 'base-200': '#181b18', 'base-300': '#252925', info: '#59d8ff', success: '#94dc7b', warning: '#ffc55c', error: '#ff6b6b' } }] },
  plugins: [daisyui],
}
