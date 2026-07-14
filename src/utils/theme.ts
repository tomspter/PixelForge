export type ThemePreference = 'system' | 'light' | 'dark'
export type ResolvedTheme = Exclude<ThemePreference, 'system'>

export const THEME_STORAGE_KEY = 'pixelforge-theme'

export function systemThemeFromMedia(): ResolvedTheme {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

export function readThemePreference(): ThemePreference {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY)
    if (stored === 'light' || stored === 'dark' || stored === 'system') return stored
  } catch {
    // A restricted WebView may disable storage; following the system remains safe.
  }
  return 'system'
}

export function saveThemePreference(preference: ThemePreference) {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, preference)
  } catch {
    // Theme switching still works for the current session without persistence.
  }
}

export function resolveTheme(preference: ThemePreference, systemTheme: ResolvedTheme): ResolvedTheme {
  return preference === 'system' ? systemTheme : preference
}

export function applyDocumentTheme(theme: ResolvedTheme) {
  document.documentElement.dataset.theme = theme === 'dark' ? 'forge' : 'forge-light'
  document.documentElement.style.colorScheme = theme
  const themeColor = document.querySelector<HTMLMetaElement>('#theme-color-meta')
  if (themeColor) themeColor.content = theme === 'dark' ? '#111311' : '#f4f6f1'
}

export function initializeDocumentTheme() {
  applyDocumentTheme(resolveTheme(readThemePreference(), systemThemeFromMedia()))
}
