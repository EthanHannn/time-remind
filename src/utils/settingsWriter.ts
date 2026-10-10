export function createSettingsWriter(
  write: (values: Record<string, string>) => Promise<void>,
  onError: (error: unknown) => void,
  onSaved: (values: Record<string, string>) => void,
) {
  let pending: Record<string, string> = {}
  let active: Promise<void> | undefined
  let timer: ReturnType<typeof setTimeout> | undefined

  async function flush(): Promise<void> {
    clearTimeout(timer)
    if (active)
      return active
    active = (async () => {
      while (Object.keys(pending).length) {
        const batch = pending
        pending = {}
        try {
          await write(batch)
        }
        catch (error) {
          pending = { ...batch, ...pending }
          onError(error)
          throw error
        }
        onSaved(batch)
      }
    })()
    try {
      await active
    }
    finally {
      active = undefined
    }
  }

  function enqueue(values: Record<string, string>) {
    pending = { ...pending, ...values }
    clearTimeout(timer)
    timer = setTimeout(() => {
      void flush().catch(() => {})
    }, 300)
  }

  return { enqueue, flush }
}
