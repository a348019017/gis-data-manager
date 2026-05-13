let toastId = 0

export function useToast() {
  function show(message, type = 'info', duration = 3000) {
    const container = document.getElementById('toast-container')
    if (!container) return
    const id = ++toastId
    const alertClass = {
      success: 'alert-success',
      error: 'alert-error',
      warning: 'alert-warning',
      info: 'alert-info',
    }[type] || 'alert-info'

    const el = document.createElement('div')
    el.id = `toast-${id}`
    el.className = `alert ${alertClass} shadow-lg`
    el.innerHTML = `<span>${message}</span>`
    container.appendChild(el)

    setTimeout(() => el.remove(), duration)
  }

  return {
    success: (msg) => show(msg, 'success'),
    error: (msg) => show(msg, 'error'),
    warning: (msg) => show(msg, 'warning'),
    info: (msg) => show(msg, 'info'),
  }
}
