import type { ShallowRef } from 'vue'
import { nextTick, onMounted, onUnmounted } from 'vue'

const modalStack: symbol[] = []
export function useModalFocus(root: Readonly<ShallowRef<HTMLElement | null>>, close: () => unknown) {
  const id = Symbol('modal')
  let previous: HTMLElement | null = null
  const controls = () => [...root.value?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex="0"]') || []].filter(element => !element.closest('[hidden], [inert]'))
  function keydown(event: KeyboardEvent) {
    if (modalStack[modalStack.length - 1] !== id || document.querySelector('dialog[open]'))
      return
    if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      void close()
    }
    else if (event.key === 'Tab') {
      const items = controls()
      const first = items[0]
      const last = items[items.length - 1]
      if (!first || !last) {
        event.preventDefault()
        root.value?.focus()
      }
      else if (event.shiftKey && (document.activeElement === first || !root.value?.contains(document.activeElement))) {
        event.preventDefault()
        last.focus()
      }
      else if (!event.shiftKey && (document.activeElement === last || !root.value?.contains(document.activeElement))) {
        event.preventDefault()
        first.focus()
      }
    }
  }
  onMounted(async () => {
    previous = document.activeElement as HTMLElement | null
    modalStack.push(id)
    window.addEventListener('keydown', keydown)
    await nextTick()
    if (modalStack[modalStack.length - 1] === id)
      (controls()[0] || root.value)?.focus()
  })
  onUnmounted(() => {
    const wasTop = modalStack[modalStack.length - 1] === id
    const index = modalStack.indexOf(id)
    if (index >= 0)
      modalStack.splice(index, 1)
    window.removeEventListener('keydown', keydown)
    if (wasTop && previous?.isConnected)
      previous.focus()
  })
}
