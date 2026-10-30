import type { Language } from './messages'

const keys = ['breakInProgress', 'breakCompleted', 'actionCompleted', 'completionMessage'] as const
const translations: Record<Language, [string, string, string, string]> = {
  'zh-CN': ['正在休息', '休息完成', '活动完成', '下一轮提醒已开始计时。'],
  'zh-TW': ['正在休息', '休息完成', '活動完成', '下一輪提醒已開始計時。'],
  'en-US': ['Resting', 'Break complete', 'Activity complete', 'Your next reminder is scheduled.'],
  'ja-JP': ['休憩中', '休憩完了', 'アクティビティ完了', '次のリマインダーを設定しました。'],
  'ko-KR': ['휴식 중', '휴식 완료', '활동 완료', '다음 알림이 예약되었습니다.'],
  'fr-FR': ['Pause en cours', 'Pause terminée', 'Activité terminée', 'Votre prochain rappel est programmé.'],
  'de-DE': ['Pause läuft', 'Pause beendet', 'Aktivität abgeschlossen', 'Die nächste Erinnerung ist geplant.'],
  'vi-VN': ['Đang nghỉ ngơi', 'Đã nghỉ xong', 'Đã hoàn thành hoạt động', 'Lời nhắc tiếp theo đã được lên lịch.'],
  'th-TH': ['กำลังพักผ่อน', 'พักผ่อนเสร็จแล้ว', 'ทำกิจกรรมเสร็จแล้ว', 'ตั้งเวลาเตือนครั้งถัดไปแล้ว'],
  'ms-MY': ['Sedang berehat', 'Rehat selesai', 'Aktiviti selesai', 'Peringatan seterusnya telah dijadualkan.'],
  'km-KH': ['កំពុងសម្រាក', 'សម្រាករួចរាល់', 'សកម្មភាពបានបញ្ចប់', 'ការរំលឹកបន្ទាប់ត្រូវបានកំណត់ពេលហើយ។'],
  'es-ES': ['Descansando', 'Descanso terminado', 'Actividad completada', 'Tu próximo recordatorio está programado.'],
  'pt-BR': ['Descansando', 'Pausa concluída', 'Atividade concluída', 'Seu próximo lembrete está agendado.'],
  'id-ID': ['Sedang beristirahat', 'Istirahat selesai', 'Aktivitas selesai', 'Pengingat berikutnya sudah dijadwalkan.'],
  'it-IT': ['Pausa in corso', 'Pausa terminata', 'Attività completata', 'Il prossimo promemoria è programmato.'],
  'ru-RU': ['Отдых', 'Перерыв завершён', 'Действие завершено', 'Следующее напоминание запланировано.'],
  'tr-TR': ['Dinleniliyor', 'Mola tamamlandı', 'Etkinlik tamamlandı', 'Sonraki hatırlatma planlandı.'],
  'ar': ['استراحة جارية', 'اكتملت الاستراحة', 'اكتمل النشاط', 'تمت جدولة التذكير التالي.'],
  'hi-IN': ['आराम जारी है', 'आराम पूरा हुआ', 'गतिविधि पूरी हुई', 'अगला रिमाइंडर निर्धारित हो गया है।'],
}

export const notificationStatusMessages = Object.fromEntries(Object.entries(translations).map(([language, values]) => [
  language,
  Object.fromEntries(keys.map((key, index) => [key, values[index]])),
])) as Record<Language, Record<(typeof keys)[number], string>>
