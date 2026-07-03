import type { PlatformCapabilities } from '../types/platform'
import { invoke } from '@tauri-apps/api/core'

export async function getPlatformCapabilities(): Promise<PlatformCapabilities> {
  return await invoke('get_platform_capabilities')
}
