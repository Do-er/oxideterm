# main → dev 同步台账

- 基线：`b4e5c653`（dev 从 main 分叉前最后一个 main 提交）
- 已同步水位：`5c186c012`（2026-10-30，#46 为止）。范围内 46 个提交全部处置完毕，#36 确认不采纳
- 清单覆盖至：`5c186c012`（2026-10-30，main 当前最新）
- 统计时间：2026-10-30
- 合并提交不计入清单：范围内共 10 个 PR / 分支合并提交，其中 9 个为纯自动合并（两父重建 tree 与合并 tree 一致）；`3a000e01b`（PR #731）含 2 行手工解决，为 `export-ble`、`ble-local-echo` 两个复选框补上控件身份参数

复现命令：

```bash
git log --no-merges <清单覆盖至>..main        # 增量提交清单
git cherry -v dev main                        # ± 对应已合入 / 未合入
git merge-tree --write-tree --merge-base=<提交>^ dev <提交>   # 单提交落到 dev 的冲突预览
```

判定规则：

- 状态 `✅` 表示已处置（含确认不采纳），`⬜` 表示待处置。状态只回答「处理过没有」，与内容是否进入 dev 无关。
- 处置 记录处理方式：`直接继承`（分叉前已在 dev）、`cherry-pick 合入`、`手工移植`、`不同步`、`未处理`。未进入 dev 但已决定不采纳的条目同样是 ✅。

## 后续维护

1. `git fetch origin main` 后执行 `git log --no-merges <清单覆盖至>..main`，把新提交追加到清单，并更新顶部「清单覆盖至」与「统计时间」；「已同步水位」只在 dev 实际吸收了 main 内容后才推进。
2. 新提交逐个执行 `git cherry -v dev main`：`-` 表示内容已在 dev，处置按来源填「直接继承」或「cherry-pick 合入」，并写 dev 侧提交；`+` 表示未进入 dev，按下面的领域规则处理。
3. 每条 `+` 先判定领域（取值见「领域决策」表），再按该领域的决策填 `处置` 并置状态：
   - 领域决策为 `吸收`：`git merge-tree` 预览无冲突即直接合入，处置填「cherry-pick 合入」（非整提交移植时填「手工移植」），并写 dev 侧提交；有冲突则保留「未处理」与 ⬜，记下冲突文件。
   - 领域决策为 `不同步`：处置填「不同步」，说明写明 dev 中的替代实现或不适用的原因，状态置 ✅。
   - 领域决策为 `逐条评估`：保留「未处理」与 ⬜，需要人工判断。
4. 新增领域时先在上表登记决策，再落到具体行。领域决策变化时批量回填其下所有「未处理」行。

## 领域决策

dev 自合并基点 `f2593f3bd` 起改动 270 个文件，集中在 `gpui-app`(100)、`i18n`(77)、`alacritty-terminal`(17)、`oxideterm-terminal`(10)、`gpui-ide`(9)、`connections`(9)。`gpui-ce`、`editor-core`、`editor-syntax`、`gpui-editor`、`ide-core`、`ide-fs`、`oxideterm-sftp` 零改动。

| 领域 | 决策 | 依据 |
| --- | --- | --- |
| 应用层 UI | 不同步 | dev 已重构 sidebar、sftp 交互、session manager；`gpui-app` 100 文件自研，main 的功能在此无宿主 |
| 编辑器 / IDE 内核 | 吸收 | dev 仅分叉 `gpui-ide` 9 文件，`editor-core`、`editor-syntax`、`gpui-editor` 零改动 |
| 渲染 / 图形底座 | 吸收 | `gpui-ce` 零改动，性能提交可直接落地 |
| 终端内核 | 吸收 | `oxideterm-terminal` 10 文件，冲突局限在 `search.rs` |
| 终端 UI | 不同步 | 纯视觉口味（滚动条宽度等），dev 自行决定 |
| 存储 / 内存 | 吸收 | 缓存上限与持久化路径，与 dev 改动正交 |
| 云同步 | 逐条评估 | dev 已重构 `gpui-cloud-sync`，需确认冲突是否为真分叉 |
| 协议 / 依赖底层 | 吸收 | `oxideterm-ssh` 1 文件；russh 定制补丁在 0.65 分支保留 |
| 平台集成（BLE） | 不同步 | dev 无 `oxideterm-ble`，属新功能而非能力升级 |
| 文档 | 跟随所属代码领域 | 代码未吸收则文档无落点 |
| 测试 | 跟随所属代码领域 | 同上 |
| CI | 已同步 | — |

## 提交清单

共 46 个提交，全部已处置。只追加，不删行。

`处置` 取值及本表分布：`直接继承` 4、`cherry-pick 合入` 33、`手工移植` 1、`不同步` 8。

| # | main 提交 | 内容 | 领域 | 状态 | 处置 | 说明 | dev 侧提交 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `ad49db3de` | 恢复启动焦点分配，统一各窗口关闭路径 | 应用层 UI | ✅ | 直接继承 | — | —
| 2 | `142150042` | 命令面板按可见名称匹配优先排序，跨分区生效 | 应用层 UI | ✅ | 直接继承 | — | —
| 3 | `e89621051` | 全屏隐藏失败时恢复窗口状态，锁定退出给出说明 | 应用层 UI | ✅ | 直接继承 | — | —
| 4 | `9abf2769d` | 命令面板保留来源搜索词与整形文本高亮 | 应用层 UI | ✅ | 直接继承 | — | —
| 5 | `80d8369b1` | 阿里云 OSS 改用 virtual-hosted 风格 S3 端点 | 云同步 | ✅ | cherry-pick 合入 | — | `ba7cf7734` |
| 6 | `428cf36a6` | IDE 默认以预览方式打开 Markdown | 编辑器 / IDE 内核 | ✅ | cherry-pick 合入 | — | `af49b1bbf` |
| 7 | `ce5761350` | 活动会话新增按 host/user 分组的第三种视图 | 应用层 UI | ✅ | 不同步 | 以 `ActiveSessionSidebarViewMode::Host` 增加第三个视图，该枚举已被 dev 的 `5cf754bd5` 整块删除（sidebar.rs 现只剩 `SidebarSection`、`ContextSidebarPanel`、`ContextSidebarTool`、`SessionActionVariant`）。dev 替代实现：活动会话为单一树视图，`sidebar/sessions.rs` 用 `node_router.flatten_tree()` 展平、`render_session_tree_child(depth, …)` 画树轨，跳板父子关系由层级缩进表达，无需合成 host/user 行。`session_group_expanded` 为运行时字段，不同步不影响持久化 | — |
| 8 | `7299a9838` | 终端 scrollback 改为文本 run + 共享属性存储 | 终端内核 | ✅ | cherry-pick 合入 | — | `f7673e0ce` |
| 9 | `20da6d5cf` | 分组视图中区分 SSH 端口与跳板路由展示 | 应用层 UI | ✅ | 不同步 | #7 的显示修复，无独立宿主：`session_group_connection_name`、`render_session_group_member_label`、`ActiveSessionSidebarViewMode::Host` 均只由 #7 引入，dev 零引用。两项改进 dev 已具备：端口贴在截断名称后（`sessions.rs` 的 `port_text`）区分同名不同端口；跳板路径由树层级表达。唯一缺失的跳板路径 tooltip 已确认不需要 | — |
| 10 | `978b70fc3` | CI 去重 Linux 检查，跳过 draft 与文档类重任务 | CI | ✅ | cherry-pick 合入 | — | `886ed5330` |
| 11 | `c05a8f665` | push 时不再重复执行平台检查 | CI | ✅ | cherry-pick 合入 | — | `07ee7e0c5` |
| 12 | `8631ffaaa` | SFTP 树视图、终端目录同步、拖放上传、路径书签、连接即开 SFTP | 应用层 UI | ✅ | 不同步 | 73 个文件。`git merge-tree` 预览仅 3 处冲突：`sidebar/sessions.rs`、`session_manager/views.rs`（dev 已删除 main 的 Active Session 焦点/面包屑模型）、`new_connection/ssh_flow/save.rs`（`open_sftp_on_connect` 参数）。SFTP 侧 `sftp/{tree,bookmarks,file_list,menus}.rs` 在 dev 与合并基点一致，可原样落地。不采纳的原因是功能取舍：dev 侧 SFTP 使用需求低，故不承担这三处会话侧边栏改写 | — |
| 13 | `a04862646` | ProxyCommand 辅助进程在 Windows 上不弹控制台窗口 | 协议 / 依赖底层 | ✅ | cherry-pick 合入 | — | `d74d9ceb7` |
| 14 | `5097dfdfd` | SFTP 树循环防护，操作路由到安全目标 | 应用层 UI | ✅ | 不同步 | 依赖 #12；`sftp/tree.rs` 在 dev 中不存在，循环防护无宿主。dev 若需要树视图应连同 cycle guard 一起自研 | — |
| 15 | `f0c9e3975` | Markdown 预览支持选中与复制文本 | 编辑器 / IDE 内核 | ✅ | cherry-pick 合入 | — | `726a29de4` |
| 16 | `e1aae466c` | 测试读取 ProxyCommand 管道完整响应 | 测试 | ✅ | cherry-pick 合入 | — | `d38ea0368` |
| 17 | `63bfd0c77` | 新增 `oxideterm-ble` crate，含扫描、连接、会话、导入导出与新建连接表单 | 平台集成（BLE） | ✅ | 不同步 | 冲突 `connections/store/tests.rs`、`new_connection/form_view/{field_controls,form_modal}.rs`、`ssh_flow/conversion.rs`、`session_manager/views.rs` 及 11 个 `locales/*/modals.json`。BLE 是新增平台能力而非底层升级，dev 目前无此 crate；如后续要做应在 dev 侧独立实现，不从 main 嫁接 | — |
| 18 | `8ef69aa50` | 编辑器共享文本快照，限制超长行的布局内存 | 编辑器 / IDE 内核 | ✅ | cherry-pick 合入 | 零冲突。#41、#44、#45 的前置；引入 `SharedText` 快照与超长行布局上限 | `fd26e10c4` |
| 19 | `e98b499e1` | 图片缓存设置上限，按显示分辨率解码 | 渲染 / 图形底座 | ✅ | cherry-pick 合入 | 零冲突。是 #28、#34 的前置。新增 `oxideterm-image-decoder` crate、`gpui-ce/src/elements/{image_cache/bounded,img/{decoded_size,sized,streaming}}.rs`；`sidebar/ai/images.rs` 改用 `image_cache(retain_recent(…))` 包住原 `img` | `1330af42d` |
| 20 | `8bd29f92e` | 回收非活动编辑器与 Markdown 预览缓存 | 编辑器 / IDE 内核 | ✅ | cherry-pick 合入 | 依赖 #18、#44。新增 `gpui-markdown/src/render/text_index.rs`。唯一冲突 `gpui-ide/surface/editor_actions.rs::create_editor`：上游 diff 上下文仍含 #44 已替换的 `TextEditorView::new`，取 HEAD 的 `from_shared_text`，保留 `set_render_cache_active(cache_active, …)`。是 #46 的前置 | `5c43eaaf2` |
| 21 | `ab5b2ecfd` | 关闭 SFTP 预览与隐藏知识库时释放缓存 | 存储 / 内存 | ✅ | cherry-pick 合入 | 零冲突。是 #46 的前置 | `90eefeba2` |
| 22 | `1d008f522` | 限制数据库缓存，向量索引改为流式持久化 | 存储 / 内存 | ✅ | cherry-pick 合入 | 零冲突 | `acfcb88f3` |
| 23 | `fe9862cc9` | WSL 帧像素传输去掉一次多余拷贝 | 渲染 / 图形底座 | ✅ | cherry-pick 合入 | 零冲突 | `b07b7eeb0` |
| 24 | `466ee67e0` | 回收空闲 element arena 容量 | 渲染 / 图形底座 | ✅ | cherry-pick 合入 | 零冲突 | `3b023fc9e` |
| 25 | `84d0c483d` | 终端滚动条宽度与命中区收窄到 8px | 终端 UI | ✅ | cherry-pick 合入 | 零冲突，但属视觉口味；dev 的 `SCROLLBAR_WIDTH` 仍为 10.0，宽度由 dev 自行决定 | — |
| 26 | `df7538086` | 新增原生编辑器内存优化与基准测试文档 | 文档 | ✅ | cherry-pick 合入 | 零冲突，新增 `docs/development/native-editor-memory.md`（748 行）。#29、#41、#45 的文档前置 | `795f7afbc` |
| 27 | `443b22bd7` | 知识库集合统计移出渲染路径并缓存 | 存储 / 内存 | ✅ | cherry-pick 合入 | 零冲突 | `345effb29` |
| 28 | `34b8ba7d4` | 图片上传共享像素，帧预取与主流程分离 | 渲染 / 图形底座 | ✅ | cherry-pick 合入 | 依赖 #19。原 modify/delete 与内容冲突仅因 #19 未合入；按 #19 → #28 顺序后零冲突 | `02da23dc9` |
| 29 | `dbb6013a1` | 补充有界缓存性能改进文档 | 文档 | ✅ | cherry-pick 合入 | 依赖 #26，插入 Ninth batch 一节 | `101f1669f` |
| 30 | `f11dff350` | SSH fork 升级到 russh 0.65.0 | 协议 / 依赖底层 | ✅ | cherry-pick 合入 | 零文本冲突。dev 原固定 fork rev `df41ee42`（0.63.0，compression-flush 补丁），该 rev 是 fork `origin/main` `e115f85`（「Upgrade to russh 0.65.0 while preserving OxideTerm patches」）的祖先，定制补丁在 0.65 中保留。改动限于 `Cargo.toml`、`Cargo.lock`、`THIRD_PARTY_NOTICES.md` | `efa7d172b` |
| 31 | `82e7baa60` | 回收隐藏布局，搜索文本共享 | 终端内核 | ✅ | cherry-pick 合入 | 唯一冲突 `workspace/root/render.rs`：该文件全文仅 1 行改动（新增 `sync_terminal_render_caches(cx)`），冲突纯属上下文偏移——dev 的 modal 收集块已重构、与上游行号错位。是 #33 的前置 | `d54f49ad8` |
| 32 | `b3aae458f` | 仪表盘卡片对齐，移除未使用的连接池容量字段 | 应用层 UI | ✅ | cherry-pick 合入 | 零冲突。删除的 `ConnectionPoolConfig::max_connections` 与 `monitor.capacity` 键在 dev 均已零引用；构造点 `root/init.rs` 未设置该字段。11 个语言包的 `capacity` 键同步删除、「`overview_pool_usage`」保留且仍被使用，键集合一致。附带一个 129 连接的注册表回归测试 | `c94ff5e7b` |
| 33 | `1ebbca767` | 限制图片驻留，压缩搜索坐标存储 | 终端内核 | ✅ | cherry-pick 合入 | 依赖 #31。原 `terminal/src/search.rs` 差异恰为 #31 的 60/36 行改动；#31 合入后零冲突 | `7cb2b6eff` |
| 34 | `889ddbc49` | 恢复图片动画整帧复用与截止时间调度 | 渲染 / 图形底座 | ✅ | cherry-pick 合入 | 依赖 #19、#28。原冲突均因两者未合入；按 #19 → #28 → #34 顺序后零冲突。合入前已核对 `gpui-ce` 图片文件与上游 `889ddbc49^` 逐字节一致，故本节删除 `img/streaming.rs` 不会丢失 #28 的改动 | `b888a7ed1` |
| 35 | `f90fc324d` | BLE 设备身份保持，取消过期连接尝试 | 平台集成（BLE） | ✅ | cherry-pick 合入 | 依赖 #17。`oxideterm-ble/{Cargo.toml,src/lib.rs}`、`new_connection/ble_flow.rs`、`form_view/ble_fields.rs`、`session_manager/ble_actions.rs`、`terminal/src/session/ble.rs` 在 dev 中不存在；内容冲突 `Cargo.lock`、`connections/store/connection_store.rs`、`new_connection.rs`、`form_state.rs`、`standalone_connections.rs` 及 11 个 `locales/*/connections.json` | — |
| 36 | `b317a9ff8` | 云同步预览与传输遵循配置文件删除选择 | 云同步 | ✅ | 不同步 | 三点决定不合入。**一、修复点无输入**：dev 仅四类墓碑（`plugin_tombstones`、`connection_tombstones`、`local_terminal_tombstones`、`ftp_tombstones`），前两类 `selection.rs:32` 已过滤、`connection_tombstones` 不经快照；唯一相关者 FTP 墓碑嵌在 `StandaloneSftpProfilesSyncSnapshot` 内，而 standalone SFTP / FTP 在 dev 三处均无逐项过滤入口（`sync_v3/mapping.rs:330` 直接 `scope.sync_connections`；`operation/local_file.rs:106`、`operation/upload_plan.rs:209` 整快照直出；`preview.rs` 的 `CloudSyncLocalFieldDiffSnapshot` 无 standalone SFTP 段），故新增 `filter_standalone_sftp_deletions` 等同 `retain` 全集。**二、其余约 470 行依赖 #17**：五个 `*ProfilesSyncSnapshot` 在 dev 无 `tombstones` 字段，`record_count() + tombstones.len()` 无法编译；14 处冲突亦全部源自 #17 插入的 BLE 分支错位。**三、未触及实时路径**：`git show --name-only` 无 `sync_v3` 文件，dev 同步只走 `deliver_causal_sync` → `prepare_sync_with_direction` → `ConfigurationView::collect`，而 v3 的删除传播本就正常（`replica.rs:141` 的 `capture_local` 对 previous 有 presence、current 无 presence 的选中资源写 `false`；`resolved_sync.rs` 整块赋值使缺席即删除）。真实缺口是 `.oxide` 导入不传播 profile 删除（FTP 除外），只能由 #17 的 profile 级墓碑模型补，而 #17 已判 `不同步`，属已接受的功能缺口 | — |
| 37 | `f46bff4c2` | 复选框动画状态按控件身份隔离 | 应用层 UI | ✅ | 不同步 | 冲突 `new_connection/form_view/{field_controls,form_modal,ftp}.rs`、`session_manager/views.rs`、`settings/{appearance,privilege_credentials_page}.rs`。控件身份参数是 main 侧表单重构的产物，dev 的表单结构已不同；同类问题 dev 若存在应按 dev 结构修 | — |
| 38 | `335cc527d` | 移除无上限的整工作区输入测试夹具 | 测试 | ✅ | cherry-pick 合入 | 零冲突；删除 `ide.rs` 的 `inline_ide_text_input_does_not_fall_through_to_terminal`，已确认 dev 无其他引用 | `f3ec4762b` |
| 39 | `d569bd5d8` | 本地终端协议回复不再等待 UI | 终端内核 | ✅ | cherry-pick 合入 | 零冲突 | `b15b2381f` |
| 40 | `3321229af` | 串口会话保留 OSC 控制序列 | 终端内核 | ✅ | 手工移植 | 手工移植 `session/serial.rs` 部分：`pending_escape: bool` 改为 `SerialEscapePrefix` 三态机，新增 `Osc` 态使 `ESC ]` 的数字化选择器原样交给终端解析器，删除 `append_serial_escape_pair`；新增 `serial_shell_control_sequences_do_not_leak_into_prompt_text`，并把 `serial_boot_text_survives_unfinished_osc_noise` 改为按分块参数化。同提交对 `raw_stream.rs` 的 6 行注释改写跳过——该文件由 #17 创建，dev 中不存在 | `96f9fab4d` |
| 41 | `f3bbe200d` | 编辑器后台搜索压缩，编辑存储共享 | 编辑器 / IDE 内核 | ✅ | cherry-pick 合入 | 依赖 #18、#26。原文档冲突仅因 #26、#29 尚未合入；按 #18 → #26 → #29 → #41 顺序后零冲突。新增 `editor-core/src/search.rs` 的 `find_all_controlled` | `baf0644ce` |
| 42 | `9e4ab3a5f` | 修复 BLE 连接无法修改分组，新建 BLE 连接支持选择分组 | 平台集成（BLE） | ✅ | 不同步 | 依赖 #17；`new_connection/form_view/ble_fields.rs` 在 dev 中不存在 | — |
| 43 | `89f8fac6a` | 知识库 UI 定时任务改用 GPUI scheduler 计时器 | 存储 / 内存 | ✅ | cherry-pick 合入 | 零冲突；`knowledge.rs` 三处 `Timer::after` 改为 GPUI scheduler 计时器 | `a42d6ce55` |
| 44 | `a28666915` | 编辑器文本快照共享：批量编辑单次应用、撤销保留切片、语法解析从 chunk 读入，IDE 保存沿用共享文本 | 编辑器 / IDE 内核 | ✅ | cherry-pick 合入 | 依赖 #18、#41。42 个文件，新增 `editor-syntax/src/source.rs`（`TextSource`）。唯一真实冲突 `gpui-ide/surface/editor_actions.rs::create_editor`：取上游的 `from_shared_text` + `cache_active`（来自已吸收的 #6 `is_markdown_preview`）；其余差异均为 #20 的 `sync_tab_render_caches`/markdown 缓存释放，不在本条范围 | `96bb41503` |
| 45 | `7c5d4aae2` | 折叠行映射压缩为区间记录，替代逐行记录 | 编辑器 / IDE 内核 | ✅ | cherry-pick 合入 | 依赖 #18、#26。合入 #18 与 #26 后零冲突 | `3e5c4ee5a` |
| 46 | `5c186c012` | Markdown 解析树共享所有权，块高度累积索引与布局复用 | 编辑器 / IDE 内核 | ✅ | cherry-pick 合入 | 依赖 #20、#21。`gpui-markdown/{layout,navigation}.rs` 与 `sftp/runtime.rs` 的冲突均随两条前置落地消失，`docs/development/native-editor-memory.md` 为纯追加 67 行，全条零冲突合入。引入 `MarkdownDocument` 的 `Arc` 共享、`BlockHeights` 累积索引、借用式 `MarkdownLayoutItem<'a>`、`MarkdownMeasurements::prepare` 内建布局，以及 `gpui-app` 的 `MarkdownPreviewCache<T>` 弱属主缓存（关闭对话框与切源码模式时释放） | `bf6b36b5b` |
