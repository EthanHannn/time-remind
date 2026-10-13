// IPC requests cannot be cancelled on the native side. Stop awaiting them so
// a late response cannot overwrite a newer retry's result.
export function withTimeout<T>(request: Promise<T>, message: string, signal: AbortSignal): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined
  let abort: () => void
  const deadline = new Promise<never>((_, reject) => {
    abort = () => reject(new DOMException('Request cancelled', 'AbortError'))
    if (signal.aborted) {
      abort()
      return
    }
    signal.addEventListener('abort', abort, { once: true })
    timer = setTimeout(() => reject(new DOMException(message, 'TimeoutError')), 15_000)
  })
  return Promise.race([request, deadline]).finally(() => {
    clearTimeout(timer)
    signal.removeEventListener('abort', abort)
  })
}
