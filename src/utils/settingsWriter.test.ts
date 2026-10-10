import { expect, it, vi } from 'vitest'
import { createSettingsWriter } from './settingsWriter'

it('coalesces rapid input and serializes newer changes behind an in-flight write', async () => {
  let resolve!: () => void
  const write = vi.fn().mockImplementationOnce(() => new Promise<void>((done) => {
    resolve = done
  })).mockResolvedValue(undefined)
  const writer = createSettingsWriter(write, vi.fn(), vi.fn())
  writer.enqueue({ sound_volume: '10' })
  writer.enqueue({ sound_volume: '20' })
  const saving = writer.flush()
  writer.enqueue({ sound_volume: '30' })
  expect(write).toHaveBeenCalledTimes(1)
  expect(write).toHaveBeenLastCalledWith({ sound_volume: '20' })
  resolve()
  await saving
  expect(write).toHaveBeenLastCalledWith({ sound_volume: '30' })
  await writer.flush()
})

it('preserves failed values for explicit retry without overwriting newer input', async () => {
  const write = vi.fn().mockRejectedValueOnce('disk full').mockResolvedValue(undefined)
  const onError = vi.fn()
  const onSaved = vi.fn()
  const writer = createSettingsWriter(write, onError, onSaved)
  writer.enqueue({ theme: 'dark', sound_volume: '0' })
  await expect(writer.flush()).rejects.toBe('disk full')
  expect(onError).toHaveBeenCalledWith('disk full')
  expect(onSaved).not.toHaveBeenCalled()
  writer.enqueue({ theme: 'light' })
  await writer.flush()
  expect(write).toHaveBeenLastCalledWith({ theme: 'light', sound_volume: '0' })
})
