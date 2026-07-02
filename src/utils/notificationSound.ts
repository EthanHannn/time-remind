export type NotificationSoundPreset = 'soft' | 'bright' | 'calm' | 'anime' | 'arcade'

declare global {
  interface Window {
    webkitAudioContext?: typeof AudioContext
  }
}

interface PlayNotificationSoundOptions {
  preset: NotificationSoundPreset
  volume: number
}

interface MelodyNote {
  frequency: number
  start: number
  duration: number
}

interface MelodyProfile {
  wave: OscillatorType
  gain: number
  notes: MelodyNote[]
}

const soundProfiles: Record<NotificationSoundPreset, MelodyProfile> = {
  soft: {
    wave: 'sine',
    gain: 0.2,
    notes: [
      { frequency: 659, start: 0, duration: 0.18 },
      { frequency: 784, start: 0.18, duration: 0.2 },
      { frequency: 988, start: 0.4, duration: 0.28 },
    ],
  },
  bright: {
    wave: 'triangle',
    gain: 0.22,
    notes: [
      { frequency: 784, start: 0, duration: 0.14 },
      { frequency: 988, start: 0.15, duration: 0.14 },
      { frequency: 1175, start: 0.3, duration: 0.16 },
      { frequency: 1568, start: 0.48, duration: 0.24 },
    ],
  },
  calm: {
    wave: 'sine',
    gain: 0.18,
    notes: [
      { frequency: 523, start: 0, duration: 0.24 },
      { frequency: 659, start: 0.26, duration: 0.28 },
      { frequency: 784, start: 0.58, duration: 0.36 },
    ],
  },
  anime: {
    wave: 'triangle',
    gain: 0.22,
    notes: [
      { frequency: 659, start: 0, duration: 0.12 },
      { frequency: 784, start: 0.13, duration: 0.12 },
      { frequency: 988, start: 0.26, duration: 0.16 },
      { frequency: 880, start: 0.44, duration: 0.12 },
      { frequency: 1175, start: 0.58, duration: 0.24 },
      { frequency: 1319, start: 0.86, duration: 0.3 },
    ],
  },
  arcade: {
    wave: 'square',
    gain: 0.13,
    notes: [
      { frequency: 523, start: 0, duration: 0.1 },
      { frequency: 659, start: 0.12, duration: 0.1 },
      { frequency: 784, start: 0.24, duration: 0.1 },
      { frequency: 1047, start: 0.36, duration: 0.14 },
      { frequency: 784, start: 0.54, duration: 0.1 },
      { frequency: 1047, start: 0.66, duration: 0.22 },
    ],
  },
}

export function playNotificationSound(options: PlayNotificationSoundOptions) {
  try {
    const AudioContextClass = window.AudioContext || window.webkitAudioContext
    if (!AudioContextClass) {
      return
    }

    const context = new AudioContextClass()
    const masterGain = context.createGain()
    const safeVolume = Math.min(Math.max(options.volume, 0), 100) / 100
    const profile = soundProfiles[options.preset] ?? soundProfiles.soft
    const startAt = context.currentTime + 0.02
    const endAt = profile.notes.reduce((max, note) => Math.max(max, note.start + note.duration), 0)

    masterGain.gain.setValueAtTime(safeVolume * profile.gain, context.currentTime)
    masterGain.connect(context.destination)

    for (const note of profile.notes) {
      const oscillator = context.createOscillator()
      const noteGain = context.createGain()
      const noteStart = startAt + note.start
      const noteEnd = noteStart + note.duration

      oscillator.type = profile.wave
      oscillator.frequency.setValueAtTime(note.frequency, noteStart)
      noteGain.gain.setValueAtTime(0.0001, noteStart)
      noteGain.gain.exponentialRampToValueAtTime(1, noteStart + 0.015)
      noteGain.gain.exponentialRampToValueAtTime(0.0001, noteEnd)
      oscillator.connect(noteGain)
      noteGain.connect(masterGain)
      oscillator.start(noteStart)
      oscillator.stop(noteEnd + 0.02)
    }

    window.setTimeout(() => {
      void context.close()
    }, Math.ceil((endAt + 0.18) * 1000))
  }
  catch {
    // 音频播放失败不影响提醒窗口流程
  }
}
