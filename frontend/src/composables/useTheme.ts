import { ref, watchEffect } from 'vue'

export function useTheme() {
  const theme = ref<'light' | 'dark'>(
    (localStorage.getItem('ws-theme') as 'light' | 'dark') || 'light'
  )

  watchEffect(() => {
    document.documentElement.dataset.theme = theme.value
    localStorage.setItem('ws-theme', theme.value)
  })

  function toggleTheme() {
    theme.value = theme.value === 'light' ? 'dark' : 'light'
  }

  return { theme, toggleTheme }
}
