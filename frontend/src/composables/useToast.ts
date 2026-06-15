import { ref } from 'vue'

export interface Toast {
  id: number
  title: string
  message: string
  type: 'success' | 'error' | 'warning' | 'info'
}

const toasts = ref<Toast[]>([])
let nextId = 0

export function useToast() {
  function show(title: string, message = '', type: Toast['type'] = 'success') {
    const id = nextId++
    toasts.value.push({ id, title, message, type })
    setTimeout(() => {
      toasts.value = toasts.value.filter(t => t.id !== id)
    }, 3200)
  }

  function showToast(type: Toast['type'], title: string, message = '') {
    show(title, message, type)
  }

  return { toasts, show, showToast }
}
