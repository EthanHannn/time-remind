import type { Language } from './messages'

const keys = ['reminders', 'pauseAll', 'resumeAll', 'edit', 'save', 'close', 'soft', 'minimal', 'illustrated', 'snoozeFor', 'snoozeOptions', 'collapse', 'expand', 'startAction'] as const
const translations: Record<Language, string[]> = {
  'zh-CN': ['我的提醒', '暂停全部提醒', '恢复全部提醒', '编辑提醒', '保存修改', '关闭', '轻盈陪伴', '系统简洁', '插画陪伴', '稍后 {minutes} 分钟', '选择延后时长', '收起计时', '展开计时', '开始行动'],
  'zh-TW': ['我的提醒', '暫停全部提醒', '恢復全部提醒', '編輯提醒', '儲存修改', '關閉', '輕盈陪伴', '系統簡潔', '插畫陪伴', '稍後 {minutes} 分鐘', '選擇延後時間', '收起計時', '展開計時', '開始行動'],
  'en-US': ['My reminders', 'Pause all reminders', 'Resume all reminders', 'Edit reminder', 'Save changes', 'Close', 'Light companion', 'System simple', 'Illustrated companion', 'In {minutes} min', 'Choose snooze duration', 'Collapse timer', 'Expand timer', 'Start activity'],
  'ja-JP': ['リマインダー', 'すべて一時停止', 'すべて再開', 'リマインダーを編集', '変更を保存', '閉じる', '小さな相棒', 'シンプル', 'イラストの相棒', '{minutes} 分後', '延期する時間を選択', 'タイマーを縮小', 'タイマーを展開', 'アクションを開始'],
  'ko-KR': ['내 알림', '모든 알림 일시 정지', '모든 알림 재개', '알림 수정', '변경 저장', '닫기', '가벼운 동반자', '간결한 알림', '일러스트 동반자', '{minutes}분 후', '미루기 시간 선택', '타이머 접기', '타이머 펼치기', '활동 시작'],
  'fr-FR': ['Mes rappels', 'Suspendre tous les rappels', 'Reprendre tous les rappels', 'Modifier le rappel', 'Enregistrer', 'Fermer', 'Petit compagnon', 'Sobre', 'Compagnon illustré', 'Dans {minutes} min', 'Choisir le délai', 'Réduire le minuteur', 'Développer le minuteur', 'Commencer l’activité'],
  'de-DE': ['Meine Erinnerungen', 'Alle Erinnerungen pausieren', 'Alle Erinnerungen fortsetzen', 'Erinnerung bearbeiten', 'Änderungen speichern', 'Schließen', 'Kleiner Begleiter', 'Schlicht', 'Illustrierter Begleiter', 'In {minutes} Minuten', 'Aufschub wählen', 'Timer einklappen', 'Timer ausklappen', 'Aktivität starten'],
  'vi-VN': ['Lời nhắc của tôi', 'Tạm dừng mọi lời nhắc', 'Tiếp tục mọi lời nhắc', 'Sửa lời nhắc', 'Lưu thay đổi', 'Đóng', 'Bạn đồng hành nhỏ', 'Đơn giản', 'Bạn đồng hành minh họa', 'Sau {minutes} phút', 'Chọn thời gian hoãn', 'Thu gọn bộ đếm', 'Mở rộng bộ đếm', 'Bắt đầu hoạt động'],
  'th-TH': ['การเตือนของฉัน', 'หยุดการเตือนทั้งหมด', 'เริ่มการเตือนทั้งหมดอีกครั้ง', 'แก้ไขการเตือน', 'บันทึกการเปลี่ยนแปลง', 'ปิด', 'เพื่อนตัวเล็ก', 'เรียบง่าย', 'เพื่อนภาพวาด', 'อีก {minutes} นาที', 'เลือกระยะเวลาเลื่อน', 'ย่อตัวจับเวลา', 'ขยายตัวจับเวลา', 'เริ่มกิจกรรม'],
  'ms-MY': ['Peringatan saya', 'Jeda semua peringatan', 'Sambung semua peringatan', 'Sunting peringatan', 'Simpan perubahan', 'Tutup', 'Teman kecil', 'Ringkas', 'Teman bergambar', 'Dalam {minutes} minit', 'Pilih tempoh tangguh', 'Kecilkan pemasa', 'Besarkan pemasa', 'Mulakan aktiviti'],
  'km-KH': ['ការរំលឹករបស់ខ្ញុំ', 'ផ្អាកការរំលឹកទាំងអស់', 'បន្តការរំលឹកទាំងអស់', 'កែសម្រួលការរំលឹក', 'រក្សាទុកការផ្លាស់ប្តូរ', 'បិទ', 'ដៃគូតូច', 'សាមញ្ញ', 'ដៃគូរូបភាព', 'ក្នុង {minutes} នាទី', 'ជ្រើសរើសរយៈពេលពន្យារ', 'បង្រួមកម្មវិធីកំណត់ម៉ោង', 'ពង្រីកកម្មវិធីកំណត់ម៉ោង', 'ចាប់ផ្ដើមសកម្មភាព'],
  'es-ES': ['Mis recordatorios', 'Pausar todos los recordatorios', 'Reanudar todos los recordatorios', 'Editar recordatorio', 'Guardar cambios', 'Cerrar', 'Pequeño compañero', 'Sencillo', 'Compañero ilustrado', 'En {minutes} min', 'Elegir tiempo de aplazamiento', 'Contraer temporizador', 'Expandir temporizador', 'Iniciar actividad'],
  'pt-BR': ['Meus lembretes', 'Pausar todos os lembretes', 'Retomar todos os lembretes', 'Editar lembrete', 'Salvar alterações', 'Fechar', 'Pequeno companheiro', 'Simples', 'Companheiro ilustrado', 'Em {minutes} min', 'Escolher tempo de adiamento', 'Recolher temporizador', 'Expandir temporizador', 'Iniciar atividade'],
  'id-ID': ['Pengingat saya', 'Jeda semua pengingat', 'Lanjutkan semua pengingat', 'Edit pengingat', 'Simpan perubahan', 'Tutup', 'Teman kecil', 'Sederhana', 'Teman ilustrasi', 'Dalam {minutes} menit', 'Pilih durasi tunda', 'Ciutkan pengatur waktu', 'Bentangkan pengatur waktu', 'Mulai aktivitas'],
  'it-IT': ['I miei promemoria', 'Sospendi tutti i promemoria', 'Riprendi tutti i promemoria', 'Modifica promemoria', 'Salva modifiche', 'Chiudi', 'Piccolo compagno', 'Essenziale', 'Compagno illustrato', 'Tra {minutes} min', 'Scegli il rinvio', 'Riduci il timer', 'Espandi il timer', 'Inizia attività'],
  'ru-RU': ['Мои напоминания', 'Приостановить все напоминания', 'Возобновить все напоминания', 'Изменить напоминание', 'Сохранить изменения', 'Закрыть', 'Маленький помощник', 'Простой вид', 'Помощник с иллюстрацией', 'Через {minutes} мин', 'Выбрать время отсрочки', 'Свернуть таймер', 'Развернуть таймер', 'Начать действие'],
  'tr-TR': ['Hatırlatmalarım', 'Tüm hatırlatmaları duraklat', 'Tüm hatırlatmaları sürdür', 'Hatırlatmayı düzenle', 'Değişiklikleri kaydet', 'Kapat', 'Küçük arkadaş', 'Sade', 'Resimli arkadaş', '{minutes} dakika sonra', 'Erteleme süresini seç', 'Sayacı daralt', 'Sayacı genişlet', 'Etkinliği başlat'],
  'ar': ['تذكيراتي', 'إيقاف جميع التذكيرات مؤقتًا', 'استئناف جميع التذكيرات', 'تعديل التذكير', 'حفظ التغييرات', 'إغلاق', 'رفيق صغير', 'بسيط', 'رفيق مصوّر', 'بعد {minutes} دقيقة', 'اختيار مدة التأجيل', 'طي المؤقت', 'توسيع المؤقت', 'بدء النشاط'],
  'hi-IN': ['मेरे रिमाइंडर', 'सभी रिमाइंडर रोकें', 'सभी रिमाइंडर फिर शुरू करें', 'रिमाइंडर संपादित करें', 'बदलाव सहेजें', 'बंद करें', 'छोटा साथी', 'सरल रूप', 'चित्र वाला साथी', '{minutes} मिनट बाद', 'टालने का समय चुनें', 'टाइमर समेटें', 'टाइमर फैलाएँ', 'गतिविधि शुरू करें'],
}

export const interfaceMessages = Object.fromEntries(Object.entries(translations).map(([language, values]) => [
  language,
  Object.fromEntries(keys.map((key, index) => [key, values[index]])),
])) as Record<Language, Record<(typeof keys)[number], string>>
