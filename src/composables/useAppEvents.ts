import type { EventCallback, EventName, UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { onUnmounted } from 'vue'
import { withTimeout } from '../utils/withTimeout'

type Subscribe = <T>(name: EventName, callback: EventCallback<T>) => Promise<void>

// Each connection attempt owns its listeners, including registrations that
// resolve after a timeout, retry, or component unmount.
export function useAppEvents(timeoutMessage: () => string) {
  let generation = 0
  let controller = new AbortController()
  const cleanups: UnlistenFn[] = []

  function disconnect() {
    generation++
    controller.abort()
    for (const cleanup of cleanups.splice(0))
      cleanup()
  }

  async function connect(register: (subscribe: Subscribe) => Promise<unknown>) {
    disconnect()
    controller = new AbortController()
    const attempt = generation
    const subscribe: Subscribe = async <T>(name: EventName, callback: EventCallback<T>) => {
      const cleanup = await listen<T>(name, (event) => {
        if (attempt === generation)
          callback(event)
      })
      if (attempt !== generation)
        cleanup()
      else
        cleanups.push(cleanup)
    }
    try {
      await withTimeout(register(subscribe), timeoutMessage(), controller.signal)
    }
    catch (error) {
      if (attempt === generation)
        disconnect()
      throw error
    }
  }

  onUnmounted(disconnect)
  return { connect }
}
