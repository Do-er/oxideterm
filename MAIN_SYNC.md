# main → dev 同步台账

- 基线：`b4e5c653`（dev 从 main 分叉前最后一个 main 提交）
- 已同步水位：`c9c25ae62`（2026-11-02，#56 为止）。范围内 56 个提交全部处置完毕，#51 确认不采纳。#57–#79 中除 #57、#76、#79 外均已处置，水位暂不推进
- 清单覆盖至：`617d889ac`（2026-11-06，main 当前最新）
- 统计时间：2026-11-06
- 合并提交不计入清单：`995e4ac01..617d889ac` 范围内共 2 个合并提交（`2772315c1` PR #749、`16773b33a` PR #750），均为纯自动合并（两父重建 tree 与合并 tree 一致）。`5c186c012..c9c25ae62` 范围内无合并提交。再上一批 46 条范围内共 10 个 PR / 分支合并提交，其中 9 个为纯自动合并；`3a000e01b`（PR #731）含 2 行手工解决，为 `export-ble`、`ble-local-echo` 两个复选框补上控件身份参数

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

共 79 个提交。#1–#56 全部已处置，#57–#79 中 20 个已处置、3 个待处理。只追加，不删行。

`未处理` 的 3 条及依赖：#57 历史建议、#76 BLE、#79 2.3.1 发布。

`处置` 取值及本表分布：`直接继承` 4、`cherry-pick 合入` 52、`手工移植` 11、`不同步` 9、`未处理` 3。

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
| 47 | `67cf8ebcf` | 扩展 Kitty 图形生命周期与动画：有序 grid 事件、会话授权本地资源、Unicode 占位符与相对放置 | 终端内核 | ✅ | 手工移植 | 54 个文件，主体为 `alacritty-terminal/src/term/graphics{,.d}/`、`oxideterm-terminal-graphics/src/graphics/{kitty,placeholders,relative,grid}.rs`、`gpui-terminal` 的 `image_animation` 与 `paint`。手工移植原因：`session/ble.rs` 在 dev 已删除，其图形接线无落点（`graphics_alt_screen_active` 字段随 BLE 一并消失，alt-screen 跟踪改由 grid 事件 API 承担）。唯一内容冲突 `session/serial.rs` 取 dev 文本；该文件内其余的换行重排属上游 diff 自身，非本题改动。11 个语言包的 `terminal.kitty_file_transmission` 文案同步更新为「本地资源授权」语义 | `d20d5be43` |
| 48 | `a1499807e` | 图片像素以共享内存租约计数，解码预算集中到 `oxideterm-image-decoder` | 渲染 / 图形底座 | ✅ | cherry-pick 合入 | 零冲突。新增 `oxideterm-image-decoder/src/budget.rs`（像素预算与租约）与 `terminal-graphics/tests/budget.rs`；`background-media`、`render-policy`、`settings`、`image_cache`、`image_animation` 改为共用同一预算实现 | `f582826e1` |
| 49 | `9edfe67ba` | 接入 Sixel grid 生命周期与 HiDPI 渲染 | 终端内核 | ✅ | cherry-pick 合入 | 零冲突。新增 `terminal-graphics/src/graphics/sixel_grid.rs`、`docs/development/sixel-graphics-compatibility.md`；`alacritty-terminal/src/term/graphics.rs` 增加 Sixel 网格接口，`terminal/src/{local/events,local/graphics_state,local/pty,tmux,vte}` 贯通事件与重排 | `cb26727ff` |
| 50 | `a44658014` | Markdown 预览测试夹具关闭无关的 agent 采样 | 测试 | ✅ | cherry-pick 合入 | 零冲突，单文件 5 行。测试跟随其所属的编辑器领域，前置 #46 已在 dev | `a8a3e25c6` |
| 51 | `544e76e8d` | 发布 OxideTerm 2.3.0：版本号、2.3.0 变更日志、内存对比图资产与生成脚本 | 文档 | ✅ | 不同步 | 三处不适用。**版本号**：`Cargo.toml`/`Cargo.lock` 改回 2.3.0，dev 自 `b770e7f4f` 起为 3.0.0，`git merge-tree` 预览在 `Cargo.toml`、`Cargo.lock` 冲突。**变更日志**：220 行 2.3.0 章节描述 main 已发布版本，含 dev 不存在的 BLE 终端（PR #731、#735），照搬会把 dev 不会发布的内容写进 dev 的发布说明；dev 最新章节为 2.2.6，3.0.0 章节应按 dev 实际内容另写。**资产与脚本**：`terminal-memory-2.3.0-comparison.{png,svg}` 与 `scripts/release/generate_terminal_memory_comparison.py` 服务于 main 的 v2.3.0 release 页面与固定 raw URL，数据取自 main 的基准；dev 若需要内存对比图，应按 dev 基准重新生成 | — |
| 52 | `b1ec34ad7` | 更新贡献指南与 AI 辅助开发规范 | 文档 | ✅ | cherry-pick 合入 | 零冲突，单文件 60 行。属流程文档，与 dev 代码路径无依赖 | `b7cf71752` |
| 53 | `948c5f4bb` | 背景色上保留透明 Powerline 分隔符 | 终端内核 | ✅ | cherry-pick 合入 | 零冲突。`terminal-view` 的 `element{,/layout,/paint,/style}` 增加分隔符切出与背景合成，`terminal-model`、`terminal/src/color.rs`、`local/pty.rs` 传递透明标志；新增 `layout_tests` 用例 | `cd0443c66` |
| 54 | `538fea5de` | 采样字段裁剪与输出上限恢复：仅取被消费的 Docker / 系统 / 服务 / journal 字段，超限样本读到结束标记 | 协议 / 依赖底层 | ✅ | cherry-pick 合入 | 零冲突。`connection-monitor/{docker,profiler,log,gpu/sampler}` 改为按字段选择解析，`oxideterm-ssh/{monitor,transport}` 处理输出上限与 Windows 引号，新增 `transport/command/tests.rs`；11 个语言包的 `sidebar.docker.{incomplete,output_limit}` 同步补齐 | `30e142042` |
| 55 | `995e4ac01` | AMD / Intel GPU 遥测按需选择字段 | 协议 / 依赖底层 | ✅ | cherry-pick 合入 | 零冲突，3 个文件。`gpu/provider/{amd,intel,mod}.rs` 收敛采样字段与解析分支，与 dev 的 GPU provider 布局一致 | `363643614` |
| 56 | `c9c25ae62` | 命令面板纳入全部已保存连接类型，与会话管理器共用库存与打开路径 | 应用层 UI | ✅ | 手工移植 | 3 个文件。移植 `saved_session_manager_items` 共享库存、`PaletteAction::OpenSavedProfile` 分发与全类型前缀/图标匹配，dev 侧差异：无 BLE 分支（`Ble` 变体与 `ble-profile` 前缀删除）、`open_session_manager_target(target, window, cx)` 无 sftp 覆盖参数、`SessionManagerDisplayItem::name()` 保持 `Cow<'_, str>` 返回类型、本地终端行沿用 `local_profile_display_name` 派生的 shell 标签（面板标签与搜索文本同会话管理器一致）。测试 `all_saved_connection_types_are_searchable_with_distinct_open_targets` 一并移植并去掉 BLE 夹具 | `7751dca5f` |
| 57 | `16513a74f` | 历史建议恢复屏幕输入，历史导航键允许覆盖 | 终端 UI | ⬜ | 未处理 | 冲突 `gpui-terminal/src/app.rs` 为真实分叉：dev 的 `TerminalInputTracker`（`app_command_marks.rs`）只有 `value`/`cursor_index`/`untracked`，无 `recovered_from_screen`、`screen_recovery_allowed`、`recovered_input_edited`；`app.rs` 无 `can_sync_working_directory`，`TerminalShortcut` 无 `SuggestPrevious`/`SuggestNext`。上游改动围绕其「从屏幕恢复输入」模型，dev 侧无对应宿主，按终端 UI 领域倾向不同步；如只需「历史导航键可覆盖」，应作为 dev 独立需求另立 | — |
| 58 | `bbff798ee` | macOS 隐藏与最小化快捷键 | 应用层 UI（窗口平台） | ✅ | cherry-pick 合入 | 零冲突。`platform.rs` 在 dev 与合并基点逐字节一致，`gpui` 已提供 `hide()`、`hide_other_apps()`、`minimize_window()`。`platform.rs` 新增 `Hide`/`HideOthers`/`Minimize`/`MinimizeAll` 动作与 App/Window 菜单项，`keybindings.rs` 以 `!KeybindingRecording` 上下文注册 cmd-h/cmd-alt-h/cmd-m/cmd-alt-m，录音期间自动让位 | `e413c2c00` |
| 59 | `99fec8b27` | Docker CLI 20.10 之前的状态解析兼容 | 协议 / 依赖底层 | ✅ | cherry-pick 合入 | 零冲突，单文件。`ps` 模板去掉 `.State`，改从 `inspect` 已有样本取 `State.Status`，新增 `docker_state_from_status` 在 inspect 不可用时按状态文本推导 | `6575a8e46` |
| 60 | `af468f4c2` | SFTP 界面字号随应用 UI 缩放 | 应用层 UI | ✅ | 手工移植 | `layout.rs` 的 `SftpUiMetrics`/`sftp_ui_metrics()` 与 `sftp.rs` 的 `sftp_file_list_virtual_spec(row_height)`、`SftpWorkspaceEntity::file_row_height` 直接摘取；其余文件按 dev 现存渲染路径逐处替换，`ui.size()` 在默认 `ui_text_xs == SFTP_TEXT_XS` 时折叠为原值。`sftp/tree.rs` 为 modify/delete，dev 无该文件（#12 判不同步），跳过；`file_list.rs` 的树模式行渲染与 `simulate_mouse_down` 键盘分派测试 dev 未采用，仅保留面板行与拖拽测试的字号覆盖 | `3c2d8933a` |
| 61 | `074910e08` | SSH 延迟监控：传输层采样与命令条展示 | 协议 / 依赖底层 + 应用层 UI | ✅ | 手工移植 | 三层拆开处理。底层 `oxideterm-ssh/src/transport/latency.rs`、`runtime_entity.rs` 采样调度、`tabs/nodes.rs` 的 `LatencyUpdated` 事件直接摘取。UI 层无宿主：`render_terminal_target_indicator` 与 cwd/git/project 上下文 chip 已被 `aaad32c98` 删除、命令条改为 composer 结构，故 `context.rs`/`bar.rs`/`privilege.rs` 保留 dev 侧，另行手写 `render_terminal_ssh_latency_chip` 落到右侧动作簇（与上游 #72 最终形态同一位置），`render_terminal_command_bar` 增加 `window` 参数 | `1c894e18a` |
| 62 | `dae89e399` | 全部本地化 README 增加 Star History 图表 | 文档 | ✅ | cherry-pick 合入 | 零冲突，11 个 README 各追加 10 行 | `c43e0e499` |
| 63 | `99102cf7a` | 平铺选中会话并放大窗格 | 应用层 UI / workspace 核心 | ✅ | 手工移植 | `oxideterm-workspace` 的 `MAX_PANES_PER_TAB: 4 → 16`、`PaneNode::tiled` 及其用例、`tabs/entity.rs` 的 `zoomed_panes`/`tile_terminal_panes`/`toggle_pane_zoom`、`pane_tree.rs` 的缩放短路与 `min_*_0` 几何、`bar.rs` 广播菜单入口均按 dev 现状落位。删掉两处 dev 没有的宿主：`tiles` 里的 `self.affinities.remove`（dev 的亲和性是 `pane_window_affinities`，按 pane 而非 tab 记录）与 `detach.rs` 的 `reveal_tab_scope`（dev 已把标签范围筛选换成标签清理菜单，`TabScope` 全域不存在）。11 个 `terminal.json` 只并入 `tile_selected`/`tile_selection_hint`/`tiled_title`，dev 已无 `members_hint`。文档按 dev 实际入口改称「广播输入」「启用广播」，并把 `architecture.md`、`runtime-ownership.md` 的窗格上限同步到 16 | `003e20c1e` |
| 64 | `4f617dfe9` | 按输入分块保留 UTF-8 控制符分派 | 终端内核 | ✅ | cherry-pick 合入 | 零冲突，单文件。dev 仅额外持有 `graphics_attribute` 分支。新增跨分块 C1 控制符保持用例 | `e61c902bb` |
| 65 | `7f2288f9a` | OSC 载荷整段批量扫描并扩充流基准 | 终端内核 | ✅ | cherry-pick 合入 | 零冲突，5 个文件。`graphics/ingress.rs` 用 `memchr` 批量定位，新增 ingress 用例与三个基准的 OSC 工作负载 | `6e04c866d` |
| 66 | `5de97ca3b` | 增量绘制复用字形批次 | 渲染 / 图形底座 | ✅ | cherry-pick 合入 | 零冲突，13 个文件。`LinePaintCache` 在 dev 已由 #31 之前的 `f7673e0ce` 引入，本条补充 prepared batch 复用、`RecentCache` 行键与语义角色缓存；新增 `cached_line_paint` 集成测试与 `performance-and-benchmarking.md` 对应章节 | `f10a7e246` |
| 67 | `b11057b9c` | 标签拖放测试对齐窗格上限 | 测试 | ✅ | cherry-pick 合入 | 零冲突，仅 `tabs/entity.rs` 测试。依赖 #63 把上限提到 16，同批合入：`detached_page_drop_uses_the_destination_leaf_and_preserves_owners` 先按 `MAX_PANES_PER_TAB` 填满、再溢出一个窗格，验证超限丢弃不改变两侧布局树与会话位置，并验证删掉溢出行后能在上限处合并 | `003e20c1e` |
| 68 | `62703e53a` | 扩充原生子系统架构图与所有权流程 | 文档 | ✅ | 手工移植 | 代码零冲突，但正文夹带 main 独有能力。按文档跟随所属代码领域的原则改回 dev 实测行为后合入：删除 BLE 后端（`session/ble.rs` 与 `oxideterm-ble` 在 dev 不存在）、`MAX_PANES_PER_TAB` 由 16 改回 4、删除平铺与窗格放大（dev 只有 `move_terminal_pane_to_tab`/`combine_pages`，无 `zoomed_panes`），en 与 zh-Hans 同步 | `67f65ec53`、`9410f170b` |
| 69 | `d9ce2404c` | 开发者与用户指南修订 | 文档 | ✅ | 手工移植 | 零代码冲突。`runtime-ownership.md` 同批夹带 main 的 `MAX_PANES_PER_TAB` (16) 与 tiling 描述，改回 dev 的 4 与「拆分/合并」后合入；其余为 crate-map 入口表、CLI 同步命令说明与 IDE 访问方式，已核对 `cloud-sync sync/push/pull/apply`、`--strategy local-wins`、Pure Shell/Agent 访问、目录监控能力在 dev 均存在 | `e3a555f9f` |
| 70 | `30656e547` | 可选连续曲率圆角 | 渲染 / 图形底座 | ✅ | cherry-pick 合入 | 唯一冲突 `OXIDETERM_PATCHES.md`：dev 已有「Window level pinning and macOS hover ownership」一节，与上游「Optional continuous Bézier corners」相邻，两节并列保留。引入 `CornerShape`、`Window::set_corner_shape`、Metal/HLSL/WGSL 三后端轮廓、`CornerStyle` 设置与 11 个语言包标签 | `0494ed3c3` |
| 71 | `18af573ab` | 延迟监控可配置并显示严重度 | 协议 / 依赖底层 + 应用层 UI | ✅ | 手工移植 | 依赖 #61。`ssh_latency` 开关进 `TerminalCommandBarSettings` 与 `set_command_bar_ssh_latency`，`configure_ssh_latency` 接到 `root/init.rs` 与 `settings/surface.rs`。dev 无 `show_current_directory`/`project_tasks`/`git_status` 字段，相关设置行与搜索键未移植；严重度色调经 `StatusTone` 映射到 `MonospaceDatumTone` | `1c894e18a` |
| 72 | `d93cf3d55` | 延迟探测间隔配置与紧凑工具栏读数 | 协议 / 依赖底层 + 应用层 UI | ✅ | 手工移植 | 依赖 #61、#71。`ssh_latency_interval_secs`（默认 10，5–60 归一化）、`SettingsSelect::SshLatencyInterval` 与 `SelectAnchorId::SettingsSshLatencyInterval`、间隔选择弹层落在 `settings/controls.rs`；`SshLatencyMonitor` 改为可配间隔并加 `set_enabled` 的悬挂/排空语义 | `1c894e18a` |
| 73 | `e915656ef` | Host Tools 支持本地机器 | 协议 / 依赖底层 | ✅ | cherry-pick 合入 | 零冲突。`env_remove` 由本提交自身引入 `tty::Options`，dev 无需预先具备。新增 `connection_monitor/exec.rs`、`HostCommandTarget::{Ssh,Local}`、`oxide-local-host` 连接 ID；`remove_env` 贯通 `LocalPtyConfig` → `LocalTerminalInstance` → `pane_tree` 分屏继承 | `c3c0bfc78` |
| 74 | `168833b72` | 端口转发批量启动选中隧道 | 应用层 UI | ✅ | 手工移植 | 冲突 `session_manager/tree.rs` 取 dev 侧：会话管理器在 dev 已是侧栏抽屉，工具栏只剩搜索与折叠，上游的选中项操作岛不存在。改由 `session_manager/views.rs` 的批量右键菜单挂出「启动所选隧道」，并按其可见性把 `MANAGER_ROW_ACTION_MENU_BATCH_HEIGHT` 加上一行 `context_menu_item_height_estimate`。`ssh_tunnels.rs` 按同主机分组、共享连接、串行排队与失败清理整体摘取；`tree.rs` 涉及的上游 modal 授权续跑改由 `tunnel_start_groups` 承接 | `6361176b3` |
| 75 | `34f2200c5` | SFTP 本地修改时间悬停显示完整时间 | 应用层 UI | ✅ | cherry-pick 合入 | 零冲突，3 个文件。`format_modified` 改为返回 `Option<DateTime<Local>>` 的 `local_modified_time`，列表单元只显示日期，悬停经 `oxideterm_gpui_ui::tooltip::tooltip_view` 显示到秒 | `42cd2df88` |
| 76 | `ee557d5c3` | BLE 已保存配置重连不匹配离线设备的 BlueZ 缓存 | 平台集成（BLE） | ⬜ | 未处理 | 修改 `terminal/src/session/ble.rs`，该文件在 dev 不存在。按既有 BLE 领域决策倾向不同步 | — |
| 77 | `c592ab934` | Host Tools 同时管理 Zellij 与 Herdr 会话 | 协议 / 依赖底层 | ✅ | cherry-pick 合入 | 预览冲突仅在 `connection_monitor/entity.rs` 的 `host_command_target` 附近；按 #73 → #77 顺序应用后自动合并。新增多路复用 provider 抽象（tmux/Zellij/Herdr） | `37639553b` |
| 78 | `b22ee38c3` | Host 命令目标保持与本地进程生命周期收敛 | 协议 / 依赖底层 | ✅ | cherry-pick 合入 | 依赖 #73、#77，顺序应用后自动合并。新增 `connection_monitor/exec/windows.rs`（Job Object 进程树终止），`Cargo.toml` 增 `libc` 与 Windows 特性 | `b781dd2d0` |
| 79 | `617d889ac` | 发布 OxideTerm 2.3.1 | 文档 | ⬜ | 未处理 | 同 #51：`Cargo.toml`/`Cargo.lock` 改回 2.3.1 与 dev 的 3.0.0 冲突，changelog 描述 dev 不会发布的内容，倾向不同步 | — |
