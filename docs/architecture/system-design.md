# Time Remind - 系统设计文档

## 0. 平台范围

- 当前实现和发布验证以 Windows 10/11 为准。
- macOS 和 Linux 具备后续支持基础，但当前未完成打包、系统能力降级和实机验证。
- 打包配置预留 macOS `.app`/`.dmg` 与 Linux `.deb`/`.AppImage` 目标，正式发布仍以 Windows NSIS 为准。
- 全屏检测、锁屏检测、托盘行为和开机自启动属于平台相关能力，非 Windows 平台不得在未验证前声明为已支持。

## 1. 架构总览

```
┌─────────────────────────────────────────────────┐
│                  Tauri 应用                       │
├──────────────────────┬──────────────────────────┤
│    Frontend (Vue 3)  │    Backend (Rust)         │
│                      │                           │
│  ┌──────────────┐    │  ┌────────────────────┐  │
│  │ 主窗口 UI    │    │  │ 定时器引擎         │  │
│  │ - 提醒列表   │    │  │ - 精确计时         │  │
│  │ - 设置面板   │    │  │ - 系统事件监听     │  │
│  │ - 统计页面   │    │  │ - 休眠/唤醒感知    │  │
│  └──────────────┘    │  └────────────────────┘  │
│                      │                           │
│  ┌──────────────┐    │  ┌────────────────────┐  │
│  │ 通知弹窗     │    │  │ 数据层             │  │
│  │ (独立窗口)   │    │  │ - SQLite 存储      │  │
│  └──────────────┘    │  │ - 配置管理         │  │
│                      │  └────────────────────┘  │
│  ┌──────────────┐    │                           │
│  │ 系统托盘     │    │  ┌────────────────────┐  │
│  │ - 右键菜单   │    │  │ 系统集成           │  │
│  │ - 状态图标   │    │  │ - 托盘管理         │  │
│  └──────────────┘    │  │ - 开机自启         │  │
│                      │  │ - 单实例启动       │  │
│                      │  │ - 全屏检测         │  │
│                      │  └────────────────────┘  │
├──────────────────────┴──────────────────────────┤
│              Tauri IPC (命令/事件)                │
└─────────────────────────────────────────────────┘
```

---

## 2. 模块划分

### 2.1 Rust 后端模块

```
src-tauri/src/
├── main.rs              // 应用入口，Tauri 初始化
├── lib.rs              // 模块注册
├── app_log.rs          // 应用运行日志与异常记录
├── timer/
│   ├── mod.rs          // 定时器引擎核心
│   ├── scheduler.rs    // 调度逻辑：计算下次触发时间
│   └── system.rs       // 系统事件监听（休眠/唤醒/锁屏）
├── db/
│   ├── mod.rs          // 数据库初始化与迁移
│   ├── reminder.rs     // Reminder 表 CRUD
│   ├── log.rs          // ReminderLog 表操作
│   └── settings.rs     // Settings 读写
├── tray/
│   └── mod.rs          // 系统托盘：图标、菜单、事件
├── notification/
│   └── mod.rs          // 通知窗口管理：创建、定位、销毁
├── autostart/
│   └── mod.rs          // 开机自启注册/取消
└── commands/
    ├── mod.rs          // Tauri 命令注册
    ├── reminder.rs     // 提醒相关 IPC 命令
    ├── settings.rs     // 设置相关 IPC 命令
    └── stats.rs        // 统计相关 IPC 命令
```

**各模块职责边界：**
- `timer/` — 只管"什么时候触发"，不管"触发后做什么"
- `notification/` — 只管"创建和销毁窗口"，不管"为什么要通知"
- `db/` — 纯数据操作，无业务逻辑
- `commands/` — 胶水层，串联上述模块响应前端调用

### 2.2 Vue 前端模块

```
src/
├── App.vue
├── main.ts
├── assets/
│   └── styles/
│       ├── variables.css    // 主题变量（亮/暗）
│       └── base.css
├── components/
│   ├── reminder/
│   │   ├── ReminderCard.vue     // 单个提醒卡片
│   │   ├── ReminderList.vue     // 提醒列表
│   │   ├── ReminderForm.vue     // 新建/编辑表单
│   │   └── CountdownBadge.vue   // 倒计时显示
│   ├── notification/
│   │   └── NotifyPopup.vue      // 通知弹窗内容
│   ├── stats/
│   │   ├── DailyStats.vue       // 今日统计
│   │   └── TrendChart.vue       // 趋势图表
│   └── common/
│       ├── AppHeader.vue
│       ├── ToggleSwitch.vue
│       └── IconButton.vue
├── composables/
│   ├── useReminders.ts      // 提醒数据管理
│   ├── useSettings.ts       // 设置状态
│   ├── useTimer.ts          // 前端倒计时同步
│   └── useTheme.ts          // 主题切换
├── stores/
│   └── app.ts               // Pinia 全局状态
├── views/
│   ├── HomeView.vue         // 主页（提醒列表）
│   ├── StatsView.vue        // 统计页
│   └── SettingsView.vue     // 设置页
├── router/
│   └── index.ts
├── utils/
│   ├── ipc.ts               // Tauri invoke 封装
│   └── format.ts            // 时间格式化工具
└── i18n/
    ├── messages.ts          // 多语言文案资源
    └── index.ts             // 系统语言匹配、语言状态、持久化与翻译函数
```

---

## 3. 核心流程

### 3.1 定时器生命周期

```
应用启动
  │
  ▼
从 DB 加载所有 enabled=true 的 Reminder
  │
  ▼
为每个 Reminder 计算 next_trigger_time
  │
  ▼
┌─────────────────────────────────────┐
│  主循环（每秒 tick 一次）             │
│                                     │
│  if 全局暂停:                        │
│    → 跳过提醒触发与本地倒计时推送     │
│                                     │
│  if now >= next_trigger:            │
│    → 按顺序进入通知队列              │
│    → 单通知窗口依次展示              │
│    → 等待用户响应                    │
│      ├─ 完成 → 写入 log             │
│      │        休息提醒可进入休息倒计时 │
│      ├─ 延后 → 重算触发时间          │
│      └─ 跳过 → 写入 log             │
│    → 重算 next_trigger              │
│                                     │
│  if 系统休眠/锁屏:                   │
│    → 暂停后端调度与通知窗口本地计时  │
│                                     │
│  if 系统唤醒/解锁:                   │
│    → 按暂停时长平移所有 next_trigger │
│                                     │
│  if 应用启动且检测到系统已重启:      │
│    → 重算所有已启用提醒 next_trigger │
└─────────────────────────────────────┘
```

### 3.2 前后端通信（IPC 协议）

**Rust → Frontend（事件推送）：**
```
reminder:triggered { reminder_id, name, icon, reminder_type, message, action_enabled, action_title, action_message, action_duration_seconds }
timer:tick        { reminder_id, remaining_secs }   // 倒计时同步
system:paused     { reason }                        // 系统休眠或锁屏
system:resumed    { reason, paused_seconds }        // 系统唤醒或解锁
```

**Frontend → Rust（命令调用）：**
```
get_reminders()           → Vec<Reminder>
create_reminder(data)     → Reminder
update_reminder(id, data) → Reminder
delete_reminder(id)       → ()
toggle_reminder(id, bool) → ()

respond_reminder(id, action, hold_notification?) → ()
postpone_reminder(id, minutes)                  → ()
release_notification(id)                        → ()
toggle_all_reminders(enabled)                   → ()

get_settings()            → Settings
update_settings(data)     → Settings

get_daily_stats(date)     → DailyStats
get_trend_stats(range)    → Vec<DayStats>
export_data()             → ExportData
import_data(data, mode)   → ImportResult   // mode: replace | merge
```

**前端危险操作确认原则：**
- 删除提醒属于高风险本地数据操作，前端必须先完成用户确认，再调用 `delete_reminder(id)`。
- 确认文案需明确提示：删除后会移除该提醒计划及其相关记录。
- 取消确认时不得触发任何 IPC 调用或列表刷新。
- 删除确认应优先使用与现有模态层一致的自定义确认框，不使用系统原生确认弹窗样式。

### 3.3 通知窗口机制

```
触发提醒
  │
  ▼
检查免打扰状态 ─── 是 ──→ 静默跳过，记录日志
  │ 否
  ▼
检查演示/游戏类真全屏应用 ─── 是 ──→ 延后5分钟，重新入队
  │ 否
  ▼
创建独立无边框窗口（always_on_top）
  │
  ├─ 定位：主显示器右下角，距边缘 16px
  ├─ 动画：从底部滑入（200ms ease-out）
  ├─ 尺寸：360 x 224px（固定）
  ├─ 队列：同一时刻只展示一条通知，后续提醒进入等待队列
  ├─ 提示：窗口内展示待处理提醒数量，不展示等待队列的具体内容
  ├─ 锁定：进入行动倒计时后仍占用当前通知位，后续提醒不得抢占
  │
  ▼
等待用户操作 / 超时自动消失（默认30秒）
  ├─ 未启用行动倒计时：立即释放当前通知，展示下一条
  └─ 已启用行动倒计时：切换为倒计时提示，倒计时结束后释放下一条
  │
  ▼
销毁窗口，写入日志
```

**串行通知状态规则：**
- `idle`：当前没有展示中的提醒，可以从等待队列取下一条。
- `showing`：当前提醒正在展示普通操作界面，禁止后续提醒覆盖界面状态。
- `holding`：当前提醒已进入行动倒计时，仍然占用当前通知位，直到倒计时结束或用户手动结束。
- `queued`：后续提醒只保留在 Rust 队列中，前端不提前渲染其内容。

**切换原则：**
- 只有 Rust 侧在释放当前提醒后，才能发出下一条 `notification:show`。
- 前端收到新的 `notification:show` 时，应视为“正式切换下一条”，而不是“刷新当前条内容”。
- 若当前提醒仍处于 `showing` 或 `holding`，前端不得因为本地状态变化自行切换提醒内容。
- 延后、跳过、完成、超时都属于“结束当前提醒”的显式结果；只有在结果落库并释放通知位后，才能继续出队。

---

## 4. 数据库设计（SQLite）

```sql
CREATE TABLE reminders (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    reminder_type TEXT NOT NULL DEFAULT 'custom',
    icon TEXT NOT NULL DEFAULT 'custom',
    message TEXT NOT NULL DEFAULT '',
    interval_minutes INTEGER NOT NULL DEFAULT 60,
    break_duration_minutes INTEGER NOT NULL DEFAULT 0,
    break_notification_enabled INTEGER NOT NULL DEFAULT 0,
    action_enabled INTEGER NOT NULL DEFAULT 0,
    action_title TEXT NOT NULL DEFAULT '',
    action_message TEXT NOT NULL DEFAULT '',
    action_duration_seconds INTEGER NOT NULL DEFAULT 0,
    action_completion_mode TEXT NOT NULL DEFAULT 'auto',
    enabled INTEGER NOT NULL DEFAULT 1,
    next_trigger TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE reminder_logs (
    id TEXT PRIMARY KEY,
    reminder_id TEXT NOT NULL,
    action TEXT NOT NULL CHECK(action IN
        ('completed','postponed','skipped','timeout')),
    triggered_at TEXT NOT NULL DEFAULT (datetime('now')),
    responded_at TEXT,
    FOREIGN KEY (reminder_id) REFERENCES reminders(id)
        ON DELETE CASCADE
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX idx_logs_reminder_date
    ON reminder_logs(reminder_id, triggered_at);
CREATE INDEX idx_logs_date
    ON reminder_logs(triggered_at);
```

**关键设置项：**
- `all_reminders_paused`：托盘“全部暂停”状态，只控制调度器是否触发提醒，不修改单条提醒 `enabled`。
- `all_reminders_paused_at`：进入全部暂停的 UTC 时间，全部恢复时按暂停时长顺延仍启用提醒的 `next_trigger`。
- `temp_dnd_until`：临时免打扰到期时间，到期前到达的提醒会顺延到该时间后。

**默认数据（首次启动插入）：**
- 饮水提醒：间隔 90 分钟
- 休息提醒：间隔 60 分钟，休息 5 分钟，默认显示休息倒计时提示
- 护眼提醒：间隔 20 分钟，完成后看向远处 20 秒

**行动倒计时模型：**
- `action_enabled` 控制完成提醒后是否进入倒计时阶段。
- `action_title` 用于通知窗口倒计时标题，例如“休息中”“护眼中”“拉伸中”。
- `action_message` 用于倒计时说明文案。
- `action_duration_seconds` 统一表示倒计时时长，支持秒级和分钟级场景。
- `action_completion_mode` 第一版使用 `auto`，表示倒计时结束后自动释放通知并进入下一轮；`manual` 作为后续扩展。
- `break_duration_minutes` 与 `break_notification_enabled` 作为旧字段保留，迁移和导入导出时映射到通用 action 字段，避免破坏已有数据。

**兼容迁移：**
- 旧休息提醒迁移为 `action_enabled = true`，标题为“休息中”，时长优先取 `action_duration_seconds`，否则取 `break_duration_minutes * 60`。
- 旧护眼提醒迁移为 `action_enabled = true`，标题为“护眼中”，默认时长 20 秒。
- 旧饮水提醒迁移为 `action_enabled = false`。
- 自定义提醒默认 `action_enabled = false`，用户可在高级设置中手动开启。

---

## 5. 关键技术决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 定时器 | Rust tokio::time + 系统时钟校准 | 不依赖前端 setInterval，精度高 |
| 通知窗口 | Tauri WebviewWindow（多窗口） | 独立于主窗口，样式完全可控 |
| 通知排队 | 单通知窗口 + Rust 串行队列 | 避免提醒互相覆盖，降低多窗口焦点与定位复杂度 |
| 数据库 | rusqlite（同步） | 数据量小，无需异步，简单可靠 |
| 主题 | CSS 变量 + prefers-color-scheme | 零 JS 开销，切换即时 |
| 状态同步 | Tauri 事件系统（后端推前端） | 后端主动推送，前端被动接收 |
| 托盘 | 静态 PNG + 动态替换 | 简单，兼容性好 |
| 开机自启 | tauri-plugin-autostart | 官方插件，封装注册表细节 |
| 单实例启动 | tauri-plugin-single-instance | 重复启动时复用已有进程，避免多个调度器并行 |
| 全屏检测 | Win32 API (GetForegroundWindow + 窗口客户区边界) | Windows 已接入；非 Windows 平台当前不声明支持 |

---

## 6. 窗口管理策略

| 窗口 | 类型 | 行为 |
|------|------|------|
| 主窗口 | 普通窗口 | 关闭=隐藏到托盘，托盘双击=显示 |
| 通知弹窗 | 无边框/置顶/不可调整 | 触发时创建，响应后销毁 |
| 全屏遮罩 | 全屏/无边框/置顶 | 护眼模式专用，倒计时后自动关闭 |

**主窗口启动策略：**
- 主窗口默认以隐藏状态创建，由 Rust 启动流程统一决定是否显示。
- 常规手动启动时，`setup` 完成后主动显示主窗口。
- 开机自启且携带 `--autostart` 参数时，若 `silent_start=true`，仅保留托盘与调度器，不弹出主窗口。
- 单实例重复启动时，只有非 `--autostart` 场景才主动唤起已有主窗口，避免静默启动误弹窗。

---

## 7. 错误处理策略

| 场景 | 处理方式 |
|------|----------|
| 数据库文件损坏 | 备份损坏文件，重建数据库，加载默认配置 |
| 定时器任务崩溃 | 监督任务记录日志并自动重启定时器工作循环 |
| 通知窗口创建失败 | 降级为系统原生通知（toast） |
| 配置值非法 | 使用默认值，日志记录警告 |
| 磁盘空间不足 | 清理超过90天的日志记录 |
