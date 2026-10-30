# main → dev 同步台账

- 基线：`b4e5c653`（dev 从 main 分叉前最后一个 main 提交）
- 已同步水位：`c96366ddf`（2026-10-30，PR #735）
- 统计时间：2026-10-30
- 合并提交不计入清单：范围内共 10 个 PR / 分支合并提交，其中 9 个为纯自动合并（两父重建 tree 与合并 tree 一致）；`3a000e01b`（PR #731）含 2 行手工解决，为 `export-ble`、`ble-local-echo` 两个复选框补上控件身份参数

复现命令：

```bash
git log --no-merges <已同步水位>..main        # 增量提交清单
git cherry -v dev main                        # ± 对应已合入 / 未合入
git merge-tree --write-tree --merge-base=<提交>^ dev <提交>   # 单提交落到 dev 的冲突预览
```

判定规则：

- ✅ 内容已完整进入 dev：祖先提交、patch-id 一致的 cherry-pick、或合并增量等价。
- ❌ 内容未进入 dev。后续无论「不合入」还是「采纳吸收」，状态都保持 ❌；处置结果只记在「未合入提交处理」表。

## 后续维护

1. `git fetch origin main` 后执行 `git log --no-merges <已同步水位>..main`，把新提交追加到清单，并更新顶部「已同步水位」与「统计时间」。
2. 新提交逐个执行 `git cherry -v dev main`：`-` 为 ✅ 并填写 dev 侧 cherry-pick hash（祖先提交填「直接继承」），`+` 为 ❌ 并在「未合入提交处理」表加一行「未处理」。
3. 处置未合入提交时更新其处理状态：`不合入` 必须写明原因及 dev 中的替代实现；`采纳吸收` 必须写明实际取用的部分并填写 dev 侧提交，清单中该行 `dev 侧提交` 同步填写，状态保持 ❌。
4. 每轮核对「提交清单中 ❌ 行数 = 未合入提交处理表行数」，不一致说明台账漂移。

## 提交清单

共 43 个提交：12 个 ✅，31 个 ❌。只追加，不删行。

| # | main 提交 | 标题 | 内容 | 状态 | dev 侧提交 |
| --- | --- | --- | --- | --- | --- |
| 1 | `ad49db3de` | fix(workspace): restore startup focus and unify window closing | 恢复启动焦点分配，统一各窗口关闭路径 | ✅ | 直接继承 |
| 2 | `142150042` | fix(palette): prioritize visible name matches across sections | 命令面板按可见名称匹配优先排序，跨分区生效 | ✅ | 直接继承 |
| 3 | `e89621051` | fix(workspace): recover failed fullscreen hides and explain locked quit | 全屏隐藏失败时恢复窗口状态，锁定退出给出说明 | ✅ | 直接继承 |
| 4 | `9abf2769d` | fix(palette): preserve source search and shaped text highlighting | 命令面板保留来源搜索词与整形文本高亮 | ✅ | 直接继承 |
| 5 | `80d8369b1` | fix(cloud-sync): use virtual-hosted S3 endpoints for Alibaba OSS | 阿里云 OSS 改用 virtual-hosted 风格 S3 端点 | ✅ | `ba7cf7734` |
| 6 | `428cf36a6` | feat(ide): preview Markdown files by default | IDE 默认以预览方式打开 Markdown | ✅ | `af49b1bbf` |
| 7 | `ce5761350` | feat(sidebar): group active sessions by host and user | 活动会话新增按 host/user 分组的第三种视图 | ❌ | — |
| 8 | `7299a9838` | perf(terminal): compact scrollback with text runs and shared attributes | 终端 scrollback 改为文本 run + 共享属性存储 | ✅ | `f7673e0ce` |
| 9 | `20da6d5cf` | fix(sidebar): distinguish grouped SSH ports and jump routes | 分组视图中区分 SSH 端口与跳板路由展示 | ❌ | — |
| 10 | `978b70fc3` | ci: deduplicate Linux checks and skip heavy draft and docs jobs | CI 去重 Linux 检查，跳过 draft 与文档类重任务 | ✅ | `886ed5330` |
| 11 | `c05a8f665` | ci: avoid repeated platform checks on push | push 时不再重复执行平台检查 | ✅ | `07ee7e0c5` |
| 12 | `8631ffaaa` | feat(sftp): add sidebar tree view, terminal sync, drop uploads, and path bookmarks | SFTP 树视图、终端目录同步、拖放上传、路径书签、连接即开 SFTP | ❌ | — |
| 13 | `a04862646` | fix(ssh): run ProxyCommand helpers without a Windows console window | ProxyCommand 辅助进程在 Windows 上不弹控制台窗口 | ✅ | `d74d9ceb7` |
| 14 | `5097dfdfd` | fix(sftp): guard tree cycles and route actions to safe targets | SFTP 树循环防护，操作路由到安全目标 | ❌ | — |
| 15 | `f0c9e3975` | fix(ide): support selecting and copying Markdown preview text | Markdown 预览支持选中与复制文本 | ✅ | `726a29de4` |
| 16 | `e1aae466c` | test(ssh): read complete ProxyCommand pipe response | 测试读取 ProxyCommand 管道完整响应 | ✅ | `d38ea0368` |
| 17 | `63bfd0c77` | feat: 新增 BLE 连接支持 | 新增 `oxideterm-ble` crate，含扫描、连接、会话、导入导出与新建连接表单 | ❌ | — |
| 18 | `8ef69aa50` | perf(editor): share text snapshots and bound long-line layout memory | 编辑器共享文本快照，限制超长行的布局内存 | ❌ | — |
| 19 | `e98b499e1` | perf(images): bound caches and decode images at display resolution | 图片缓存设置上限，按显示分辨率解码 | ❌ | — |
| 20 | `8bd29f92e` | perf(ide): reclaim inactive editor and Markdown preview caches | 回收非活动编辑器与 Markdown 预览缓存 | ❌ | — |
| 21 | `ab5b2ecfd` | fix(memory): release closed SFTP previews and hidden knowledge caches | 关闭 SFTP 预览与隐藏知识库时释放缓存 | ❌ | — |
| 22 | `1d008f522` | perf(storage): limit database caches and stream vector index persistence | 限制数据库缓存，向量索引改为流式持久化 | ❌ | — |
| 23 | `fe9862cc9` | perf(graphics): transfer WSL frame pixels without an extra copy | WSL 帧像素传输去掉一次多余拷贝 | ❌ | — |
| 24 | `466ee67e0` | perf(gpui): reclaim cold element arena capacity | 回收空闲 element arena 容量 | ❌ | — |
| 25 | `84d0c483d` | style(terminal): narrow scrollbar and hit area to eight pixels | 终端滚动条宽度与命中区收窄到 8px | ❌ | — |
| 26 | `df7538086` | docs: record application memory optimizations and benchmarks | 新增原生编辑器内存优化与基准测试文档 | ❌ | — |
| 27 | `443b22bd7` | perf(knowledge): cache collection statistics outside rendering | 知识库集合统计移出渲染路径并缓存 | ❌ | — |
| 28 | `34b8ba7d4` | perf(images): share upload pixels and separate frame prefetch | 图片上传共享像素，帧预取与主流程分离 | ❌ | — |
| 29 | `dbb6013a1` | docs: record bounded-cache performance improvements | 补充有界缓存性能改进文档 | ❌ | — |
| 30 | `f11dff350` | Upgrade SSH fork to russh 0.65.0 | SSH fork 升级到 russh 0.65.0 | ❌ | — |
| 31 | `82e7baa60` | perf(terminal): reclaim hidden layouts and share search text | 回收隐藏布局，搜索文本共享 | ❌ | — |
| 32 | `b3aae458f` | fix(dashboard): align cards and remove unused pool capacity | 仪表盘卡片对齐，移除未使用的连接池容量字段 | ❌ | — |
| 33 | `1ebbca767` | perf(terminal): bound image retention and compact search coordinates | 限制图片驻留，压缩搜索坐标存储 | ❌ | — |
| 34 | `889ddbc49` | fix(images): restore full animation frame reuse and deadline scheduling | 恢复图片动画整帧复用与截止时间调度 | ❌ | — |
| 35 | `f90fc324d` | fix(ble): preserve device identity and cancel stale connection attempts | BLE 设备身份保持，取消过期连接尝试 | ❌ | — |
| 36 | `b317a9ff8` | fix(sync): honor profile deletion selection in previews and transfers | 云同步预览与传输遵循配置文件删除选择 | ❌ | — |
| 37 | `f46bff4c2` | fix(ui): isolate checkbox animation state by control identity | 复选框动画状态按控件身份隔离 | ❌ | — |
| 38 | `335cc527d` | test(ide): remove unbounded full-workspace input fixture | 移除无上限的整工作区输入测试夹具 | ❌ | — |
| 39 | `d569bd5d8` | fix(terminal): send local protocol replies without waiting for UI | 本地终端协议回复不再等待 UI 处理 | ❌ | — |
| 40 | `3321229af` | fix(terminal): preserve serial OSC control sequences | 串口会话保留 OSC 控制序列 | ❌ | — |
| 41 | `f3bbe200d` | perf(editor): compact background search and share edit storage | 编辑器后台搜索压缩，编辑存储共享 | ❌ | — |
| 42 | `9e4ab3a5f` | fix: 修复ble连接在会话管理器中无法修改分组的问题，同时在新建ble连接UI增加分组选择功能。 | 修复 BLE 连接无法修改分组，新建 BLE 连接支持选择分组 | ❌ | — |
| 43 | `89f8fac6a` | fix(knowledge): use GPUI scheduler timers for UI tasks | 知识库 UI 定时任务改用 GPUI scheduler 计时器 | ❌ | — |

## 未合入提交处理

行集合等于提交清单中所有 ❌ 行。处理完成后不删行，作为决策记录长期保留。

处理状态取值：`未处理` / `不合入` / `采纳吸收`。

| # | main 提交 | 处理状态 | 说明（原因 / 前提 / 冲突点） | dev 侧提交 |
| --- | --- | --- | --- | --- |
| 7 | `ce5761350` | 未处理 | dev 已重构 sidebar（移除左边栏、活动会话移至右侧栏），冲突文件 `sidebar.rs`、`sidebar/region.rs`、`sidebar/sessions.rs`，需人工改写而非直接套用 | — |
| 9 | `20da6d5cf` | 未处理 | 依赖 #7，dev 无分组视图代码 | — |
| 12 | `8631ffaaa` | 未处理 | 73 个文件，冲突 3 个：`new_connection/ssh_flow/save.rs`、`session_manager/views.rs`、`sidebar/sessions.rs`；其余可自动合入，新增 `sftp/{bookmarks,tree}.rs` | — |
| 14 | `5097dfdfd` | 未处理 | 依赖 #12。冲突 `sftp/actions/{menus_conflicts,navigation}.rs`、`sftp/file_list.rs`、`sftp/runtime.rs`；`sftp/tree.rs` 在 dev 中不存在 | — |
| 17 | `63bfd0c77` | 未处理 | 新增 `oxideterm-ble` crate。冲突：`connections/store/tests.rs`、`new_connection/form_view/{field_controls,form_modal}.rs`、`ssh_flow/conversion.rs`、`session_manager/views.rs` 及 11 个 `locales/*/modals.json` | — |
| 18 | `8ef69aa50` | 未处理 | 无冲突，可自动合入 | — |
| 19 | `e98b499e1` | 未处理 | 无冲突，可自动合入 | — |
| 20 | `8bd29f92e` | 未处理 | 无冲突，可自动合入；`gpui-markdown/src/render/text_index.rs` 为新增文件 | — |
| 21 | `ab5b2ecfd` | 未处理 | 无冲突，可自动合入 | — |
| 22 | `1d008f522` | 未处理 | 无冲突，可自动合入 | — |
| 23 | `fe9862cc9` | 未处理 | 无冲突，可自动合入 | — |
| 24 | `466ee67e0` | 未处理 | 无冲突，可自动合入 | — |
| 25 | `84d0c483d` | 未处理 | 无冲突，可自动合入；dev 的 `SCROLLBAR_WIDTH` 仍为 10.0 | — |
| 26 | `df7538086` | 未处理 | 无冲突，新增文档文件，可自动合入 | — |
| 27 | `443b22bd7` | 未处理 | 无冲突，可自动合入 | — |
| 28 | `34b8ba7d4` | 未处理 | 冲突（modify/delete）：`gpui-ce/src/elements/img/{sized,streaming}.rs` 在 dev 中不存在；`assets.rs`、`window.rs` 内容冲突 | — |
| 29 | `dbb6013a1` | 未处理 | 依赖 #26，`docs/development/native-editor-memory.md` 在 dev 中不存在 | — |
| 30 | `f11dff350` | 未处理 | 无文本冲突，但 dev 的 `russh` 仍固定在 fork 的 `df41ee42` rev（0.63.0），合入会替换为 0.65.0，需确认 dev 定制改动在上游仍存在 | — |
| 31 | `82e7baa60` | 未处理 | 仅 `workspace/root/render.rs` 冲突；`terminal/src/{local/pty,search}.rs`、`gpui-terminal/src/{app.rs,terminal_view/element.rs}`、`tabs/{detach,entity,state}.rs` 无冲突 | — |
| 32 | `b3aae458f` | 未处理 | 无冲突，可自动合入 | — |
| 33 | `1ebbca767` | 未处理 | 依赖 #31：`terminal/src/search.rs` 在 `1ebbca767^` 与 dev 的差异恰好是 #31 的 60/36 行改动，dev 侧该文件与 #8 的 cherry-pick `f7673e0ce` 一致 | — |
| 34 | `889ddbc49` | 未处理 | 冲突（modify/delete）：`elements/image_cache/bounded.rs`、`elements/img/{decoded_size,sized}.rs` 在 dev 中不存在，由 #19、#28 引入；内容冲突 `assets.rs`、`elements/{image_cache,img}.rs`、`platform.rs`、`window.rs`、`gpui-terminal/src/app.rs`、`docs/development/performance-and-benchmarking.md` | — |
| 35 | `f90fc324d` | 未处理 | 依赖 #17。modify/delete：`oxideterm-ble/{Cargo.toml,src/lib.rs}`、`new_connection/ble_flow.rs`、`form_view/ble_fields.rs`、`session_manager/ble_actions.rs`、`terminal/src/session/ble.rs` 在 dev 中不存在。内容冲突：`Cargo.lock`、`connections/store/connection_store.rs`、`new_connection.rs`、`new_connection/form_state.rs`、`standalone_connections.rs` 及 11 个 `locales/*/connections.json` | — |
| 36 | `b317a9ff8` | 未处理 | 冲突 10 个云同步文件（`cloud-sync/src/operation/{selection,upload_plan}.rs`、`oxide_file/transfer/{common,export}.rs`、`gpui-cloud-sync/{delivery,preview,preview/*,selection}.rs`），dev 已重构 `gpui-cloud-sync` 模块 | — |
| 37 | `f46bff4c2` | 未处理 | 冲突 `new_connection/form_view/{field_controls,form_modal,ftp}.rs`、`session_manager/views.rs`、`settings/{appearance,privilege_credentials_page}.rs` | — |
| 38 | `335cc527d` | 未处理 | 无冲突，可自动合入；dev 的 `ide.rs` 仍保留被删除的 `inline_ide_text_input_does_not_fall_through_to_terminal` | — |
| 39 | `d569bd5d8` | 未处理 | 无冲突，可自动合入 | — |
| 40 | `3321229af` | 未处理 | 冲突（modify/delete）`terminal/src/session/raw_stream.rs`（由 #17 引入，dev 不存在），另 `session/serial.rs` 内容冲突 | — |
| 41 | `f3bbe200d` | 未处理 | 冲突 `editor-core/{buffer,piece_table,search}.rs`、`gpui-editor/src/surface.rs`、`gpui-editor/surface/{performance,render,search,syntax_task}.rs`；`surface/segments.rs`、`native-editor-memory.md` 在 dev 中不存在。依赖 #18、#26 | — |
| 42 | `9e4ab3a5f` | 未处理 | 依赖 #17；`new_connection/form_view/ble_fields.rs` 在 dev 中不存在 | — |
| 43 | `89f8fac6a` | 未处理 | 无冲突，可自动合入；dev 的 `knowledge.rs` 仍在 491/503/538 行使用 `Timer::after` | — |
