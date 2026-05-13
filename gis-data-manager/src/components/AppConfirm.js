export function useConfirm() {
  return function confirm(title, message) {
    return new Promise((resolve) => {
      const dialog = document.createElement('dialog')
      dialog.className = 'modal'
      dialog.innerHTML = `
        <div class="modal-box">
          <h3 class="font-bold text-lg">${title}</h3>
          <p class="py-4">${message}</p>
          <div class="modal-action">
            <button class="btn btn-ghost cancel-btn">取消</button>
            <button class="btn btn-primary confirm-btn">确认</button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop"><button>close</button></form>
      `
      document.body.appendChild(dialog)
      dialog.showModal()

      dialog.querySelector('.cancel-btn').onclick = () => {
        dialog.close()
        dialog.remove()
        resolve(false)
      }
      dialog.querySelector('.confirm-btn').onclick = () => {
        dialog.close()
        dialog.remove()
        resolve(true)
      }
      dialog.addEventListener('close', () => {
        dialog.remove()
        resolve(false)
      }, { once: true })
    })
  }
}
