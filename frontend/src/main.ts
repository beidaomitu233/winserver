import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './stores/useDatabaseStore'

async function setupDevRuntime() {
  if (!import.meta.env.DEV || typeof window === 'undefined' || (window as any).__TAURI_INTERNALS__) return
  const { setupTauriDevMock } = await import('./dev/tauriMock')
  setupTauriDevMock()
}

setupDevRuntime().then(() => {
  const pinia = createPinia()
  const app = createApp(App)
  app.use(pinia)
  app.mount('#app')
})
