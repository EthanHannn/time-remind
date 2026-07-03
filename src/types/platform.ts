export type PlatformName = 'windows' | 'macos' | 'linux' | 'unknown'

export interface PlatformCapabilities {
  platform: PlatformName
  isVerifiedReleasePlatform: boolean
  supportsFullscreenDetection: boolean
  supportsLockDetection: boolean
  supportsTray: boolean
  supportsAutostart: boolean
  supportsSilentStart: boolean
}
