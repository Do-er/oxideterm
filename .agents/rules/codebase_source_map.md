---
trigger: always_on
glob:
description:
---

# OxideTerm Codebase Source Map & Architecture Index

本规则为 AI 提供 OxideTerm 全仓库（79 个 Crates）的**功能到源码直达索引地图**。
AI 在接收到用户的功能修改、Bug 修复、UI 调整或性能优化请求时，应优先查阅本地图，直接定位关键文件，严禁全仓库无序盲搜。

---

## 1. 架构三层分层模型

```text
┌────────────────────────────────────────────────────────────────────────┐
│ Layer 3: 顶层工作区与业务装配层 (Application & Workspace UI)            │
│ crates/oxideterm-gpui-app (单窗口装配、标签、侧栏、弹窗、全局交互)         │
├────────────────────────────────────────────────────────────────────────┤
│ Layer 2: GPUI 基础视图与组件层 (Reusable GPUI Views & Widgets)          │
│ - 终端 GPU 画布与交互: crates/oxideterm-gpui-terminal                  │
│ - 通用 UI 控件 (Button/Dialog/Input): crates/oxideterm-gpui-ui         │
│ - 背景渲染与动效: crates/oxideterm-gpui-background                     │
│ - 编辑器视图: crates/oxideterm-gpui-editor                             │
├────────────────────────────────────────────────────────────────────────┤
│ Layer 1: 纯业务、协议与算法核心层 (Headless Logic & Protocol Crates)     │
│ - 终端底层驱动: oxideterm-terminal, vte, alacritty-terminal            │
│ - 触发器算法: oxideterm-terminal-triggers                               │
│ - 连接与协议: oxideterm-ssh, oxideterm-sftp, oxideterm-forwarding       │
│ - 配置模型: oxideterm-settings, oxideterm-settings-model               │
│ - 主题资产: oxideterm-theme, oxideterm-background-media                │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. 核心功能 ➔ 源码直达索引字典

### 2.1 终端核心与交互（Terminal Core & Interactions）
| 需求场景 | 关键文件与路径 | 说明 |
| :--- | :--- | :--- |
| **滚轮/按键/缩放/手势** | `crates/oxideterm-gpui-terminal/src/app/interactions.rs` | 鼠标滚轮 `handle_scroll`、点击选择、修饰键处理 |
| **终端文本与图元绘制** | `crates/oxideterm-gpui-terminal/src/terminal_view/element/paint.rs` | DirectX/DirectWrite 字符渲染、光标、高亮选区 |
| **字符尺寸与行高度量** | `crates/oxideterm-gpui-terminal/src/terminal_ui.rs` | `TerminalMetrics`、`cell_width`、`line_height` 计算 |
| **复制、粘贴与剪贴板** | `crates/oxideterm-gpui-terminal/src/app/paste.rs` | 安全粘贴检查、OSC 52 剪贴板集成 |
| **命令输出折叠** | `crates/oxideterm-gpui-terminal/src/app/command_folding.rs` | 长命令折叠指示器、行号槽渲染 |
| **终端搜索条** | `crates/oxideterm-terminal/src/search.rs` | 终端内容正则/文本搜索、高亮导航 |
| **PTY / 本地子进程** | `crates/oxideterm-terminal/src/local/pty.rs` | Windows ConPTY、Unix PTY 伪终端派生 |
| **终端工作目录 (CWD)** | `crates/oxideterm-gpui-app/src/workspace/terminal_cwd.rs` | CWD 探测、OSC 7 解析、目录跳转 |

### 2.2 终端触发器系统（Terminal Triggers）
| 需求场景 | 关键文件与路径 | 说明 |
| :--- | :--- | :--- |
| **流式文本扫描与正则匹配** | `crates/oxideterm-terminal-triggers/src/scanner.rs` | `TerminalTriggerStream`、整词匹配、控制码过滤 |
| **触发器规则编译与数据模型** | `crates/oxideterm-terminal-triggers/src/compiler.rs`<br>`crates/oxideterm-terminal-triggers/src/model.rs` | 触发器持久化模型、正则预编译 |
| **SSH/本地终端挂载触发器** | `crates/oxideterm-terminal/src/session/ssh_parser.rs`<br>`crates/oxideterm-terminal/src/local_graphics_event_loop.rs` | 解码输出流送入 `stream.observe_bytes` |
| **触发动作执行运行时** | `crates/oxideterm-gpui-app/src/workspace/terminal_triggers_runtime.rs` | `handle_terminal_trigger_matches`、发文本/弹窗/URL |
| **触发器设置界面 UI** | `crates/oxideterm-gpui-app/src/workspace/settings/terminal_triggers.rs` | 规则列表、添加/编辑表单、整词/追加回车开关 |

### 2.3 底部快捷命令条（Command Sender）
| 需求场景 | 关键文件与路径 | 说明 |
| :--- | :--- | :--- |
| **底层发送模型与状态** | `crates/oxideterm-terminal/src/command_sender.rs` | 命令队列、状态机、多终端目标派发 |
| **工作区交互与定时器** | `crates/oxideterm-gpui-app/src/workspace/terminal_command_sender.rs` | 定时重复、按键事件、文档状态管理 |
| **UI 输入栏与控制按钮** | `crates/oxideterm-gpui-app/src/workspace/terminal_command_bar/sender.rs` | 底部输入框、运行/停止、步进调节器渲染 |

### 2.4 界面布局与视窗组织（Workspace, Tabs & Sidebar）
| 需求场景 | 关键文件与路径 | 说明 |
| :--- | :--- | :--- |
| **顶部标签栏 (TabBar)** | `crates/oxideterm-gpui-app/src/workspace/tabs/` | 标签拖拽、重命名、关闭、多窗口分离 (`detach.rs`) |
| **窗口标题栏 (TitleBar)** | `crates/oxideterm-gpui-app/src/workspace/sidebar/titlebar.rs` | 紧凑操作按钮、后台任务摘要指示 |
| **左侧与右侧活动栏** | `crates/oxideterm-gpui-app/src/workspace/sidebar/activity.rs` | 左侧会话/连接图标、右侧 AI/Host Tools 图标 |
| **会话树与已存主机列表** | `crates/oxideterm-gpui-app/src/workspace/sidebar/sessions.rs`<br>`crates/oxideterm-gpui-app/src/workspace/sidebar/connections.rs` | SSH 节点树、分组、右键连接菜单 |
| **多终端分屏网格** | `crates/oxideterm-gpui-app/src/workspace/pane_tree.rs` | 左右分屏、上下分屏、焦点切换、尺寸拖拽 |
| **全局主窗口装配根入口** | `crates/oxideterm-gpui-app/src/workspace/root/render.rs`<br>`crates/oxideterm-gpui-app/src/workspace/root/init.rs` | 整体视窗布局组装、事件总线分发 |
| **快捷命令抽屉 (Quick Commands)** | `crates/oxideterm-gpui-app/src/workspace/quick_commands_view.rs` | 常用脚本库、分组抽屉、卡片悬停预览 |
| **全局命令面板 (Ctrl+P)** | `crates/oxideterm-gpui-app/src/workspace/command_palette/` | 快速搜索、历史命令、快捷操作过滤 |

### 2.5 文件管理与传输（File Manager & SFTP）
| 需求场景 | 关键文件与路径 | 说明 |
| :--- | :--- | :--- |
| **内置本地文件管理器** | `crates/oxideterm-gpui-app/src/workspace/file_manager/` | 本地文件树、盘符切换、书签收藏、文件操作 |
| **文件快速预览 (QuickLook)** | `crates/oxideterm-gpui-app/src/workspace/file_manager/dialogs/preview.rs` | 空格键预览：Markdown、代码高亮、NerdFont字体 |
| **SFTP 远程文件传输面板** | `crates/oxideterm-gpui-app/src/workspace/sftp/` | 双栏/单栏传输、进度条、文件上传下载队列 |
| **SFTP 底层驱动** | `crates/oxideterm-sftp/` | 异步 SFTP 客户端、断点续传、通道复用 |

### 2.6 主题、外观与背景媒体（Theme & Appearance）
| 需求场景 | 关键文件与路径 | 说明 |
| :--- | :--- | :--- |
| **主题色彩与 Token 定义** | `crates/oxideterm-theme/` | 预设主题（Tokyo Night, Catppuccin, OneDark 等） |
| **亮暗主题双配置模型** | `crates/oxideterm-settings/src/model/theme.rs` | `ThemeSettings`、跟随系统/显式指定 |
| **外观设置页面 UI** | `crates/oxideterm-gpui-app/src/workspace/settings/appearance.rs` | 主题选择器、透明度调节、光标样式 |
| **背景媒体与视频壁纸播放** | `crates/oxideterm-gpui-background/src/player.rs`<br>`crates/oxideterm-background-media/` | 内存安全熔断（192MB/64MB）、MP4/GIF 硬件解码 |
| **应用内置图标与变体** | `crates/oxideterm-gpui-app/src/app_icon.rs` | 应用图标色彩变体、任务栏图标更新 |

### 2.7 网络与协议（Networking & Protocols）
| 需求场景 | 关键文件与路径 | 说明 |
| :--- | :--- | :--- |
| **SSH 连接池与节点路由** | `crates/oxideterm-ssh/` | `NodeRouter`、跳板机（JumpHost）、密钥认证 |
| **远程 Shell 目录集成** | `crates/oxideterm-terminal/src/remote_shell_integration.rs` | 远程 `.oxideterm/shell-integration/` 脚本注入 |
| **端口转发 (Port Forwarding)** | `crates/oxideterm-forwarding/` | 本地/远程端口转发、动态 SOCKS5 代理 |
| **Telnet / Mosh / 串口** | `crates/oxideterm-terminal/src/session/` | 各专用协议会话生命周期与输入输出封装 |

### 2.8 设置中心与国际化（Settings & i18n）
| 需求场景 | 关键文件与路径 | 说明 |
| :--- | :--- | :--- |
| **设置存储与持久化** | `crates/oxideterm-settings/src/store.rs` | `settings.json` 读写、校验、默认值合并 |
| **全局快捷键绑定** | `crates/oxideterm-gpui-app/src/keybindings.rs` | 全局按键映射、快捷键冲突检测与覆盖 |
| **11 国语言本地化文案** | `crates/oxideterm-i18n/locales/<locale>/` | 界面展示文本（修改文案必须 11 个语言包同步） |

### 2.9 发布与打包工程（Build & Release Scripts）
| 需求场景 | 关键文件与路径 | 说明 |
| :--- | :--- | :--- |
| **原生跨平台打包主脚本** | `scripts/release/package_native.py` | 制作便携包 `.zip` 与 Windows `.exe` 安装包 |
| **ConPTY 运行时捆绑** | `scripts/release/conpty_runtime.py` | 自动下载校验微软原生 ConPTY 动态库 |
| **版本升级与 Changelog** | `scripts/release/bump_version.py` | 全局版本号递增同步 |

---

## 3. AI 开发执行原则（必须严格遵守）

1. **先定位层级，再精确修改**：
   - 界面视觉改动 ➔ `crates/oxideterm-gpui-app/src/workspace/`
   - 终端渲染/手势 ➔ `crates/oxideterm-gpui-terminal/`
   - 底层协议/数据算法 ➔ 对应的无 UI 独立 crate。
2. **严禁破坏分层模型**：
   - 不得在纯逻辑 crate 中引入 GPUI 渲染依赖。
   - 不得在单个终端 Pane 中隐式断开全局共享的 SSH 节点。
3. **文案必须接入 i18n**：
   - 禁止在 Rust 代码中硬编码用户可见文本，必须使用 `self.i18n.t(...)`。
4. **编译安全红线**：
   - 仅允许执行 `cargo check -p oxideterm-gpui-app` 快速检查，严禁擅自执行全局 `cargo build` / `cargo test`。
