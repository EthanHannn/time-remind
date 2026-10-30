/// 提醒配置
export interface Reminder {
  id: string
  name: string
  reminder_type: string
  icon: string
  message: string
  interval_minutes: number
  break_duration_minutes: number
  break_notification_enabled: boolean
  action_enabled: boolean
  action_title: string
  template_fields?: string[] | null
  action_message: string
  action_duration_seconds: number
  action_completion_mode: 'auto' | 'manual'
  enabled: boolean
  next_trigger: string | null
  created_at: string
  updated_at: string
}

/// 创建提醒请求
export interface CreateReminderRequest {
  name: string
  reminder_type: string
  icon: string
  message: string
  interval_minutes: number
  break_duration_minutes?: number
  break_notification_enabled?: boolean
  action_enabled?: boolean
  action_title?: string
  action_message?: string
  action_duration_seconds?: number
  action_completion_mode?: 'auto' | 'manual'
  enabled?: boolean
}

/// 更新提醒请求
export interface UpdateReminderRequest {
  name?: string
  reminder_type?: string
  icon?: string
  message?: string
  interval_minutes?: number
  break_duration_minutes?: number
  break_notification_enabled?: boolean
  action_enabled?: boolean
  action_title?: string
  action_message?: string
  action_duration_seconds?: number
  action_completion_mode?: 'auto' | 'manual'
  enabled?: boolean
}

/// 定时器 tick 事件
export interface TimerTick {
  reminder_id: string
  remaining_seconds: number
}

export interface ReminderRuntime {
  state: 'disabled' | 'scheduled' | 'due' | 'queued' | 'notifying' | 'acting' | 'paused' | 'dnd'
  reason?: 'manual' | 'system' | 'temporary' | 'scheduled' | 'fullscreen' | null
  until?: string | null
}

export interface RuntimeSnapshot {
  revision: number
  all_paused: boolean
  suppression?: ReminderRuntime | null
  countdowns: Record<string, number>
  states: Record<string, ReminderRuntime>
}

export interface ReminderOverview extends RuntimeSnapshot {
  reminders: Reminder[]
}

/// 提醒触发事件
export interface ReminderTriggered {
  reminder_id: string
  name: string
  icon: string
  reminder_type: string
  break_duration_minutes: number
  break_notification_enabled: boolean
  action_enabled: boolean
  action_title: string
  action_message: string
  action_duration_seconds: number
  action_completion_mode: 'auto' | 'manual'
  message: string
}
