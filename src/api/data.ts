import type { ExportData, ImportMode, ImportResult } from '../types/data'
import { invoke } from '@tauri-apps/api/core'

/// 导出全部数据
export async function exportData(): Promise<ExportData> {
  return await invoke('export_data')
}

/// 导入全部数据
export async function importData(data: ExportData, mode: ImportMode): Promise<ImportResult> {
  return await invoke('import_data', { data, mode })
}

/// 写入文本文件
export async function writeTextFile(path: string, content: string): Promise<void> {
  return await invoke('write_file', { path, content })
}

/// 读取文本文件
export async function readTextFile(path: string): Promise<string> {
  return await invoke('read_file', { path })
}
