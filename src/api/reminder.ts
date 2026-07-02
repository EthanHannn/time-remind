import type { CreateReminderRequest, Reminder, UpdateReminderRequest } from '../types/reminder'
import { invoke } from '@tauri-apps/api/core'

/// 获取所有提醒
export async function getReminders(): Promise<Reminder[]> {
  return await invoke('get_reminders')
}

/// 创建提醒
export async function createReminder(request: CreateReminderRequest): Promise<Reminder> {
  return await invoke('create_reminder', { request })
}

/// 更新提醒
export async function updateReminder(id: string, request: UpdateReminderRequest): Promise<Reminder> {
  return await invoke('update_reminder', { id, request })
}

/// 删除提醒
export async function deleteReminder(id: string): Promise<void> {
  return await invoke('delete_reminder', { id })
}

/// 切换提醒启用/禁用
export async function toggleReminder(id: string): Promise<Reminder> {
  return await invoke('toggle_reminder', { id })
}

/// 记录提醒响应
export async function respondReminder(reminderId: string, action: string, holdNotification = false): Promise<void> {
  return await invoke('respond_reminder', { reminderId, action, holdNotification })
}
