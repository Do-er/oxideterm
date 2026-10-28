# main → dev 同步台账

- 统计范围：`b4e5c653972bb0966b24dfddcde23b888bda6ebc..main`（不含基线提交本身）
- 基线：`b4e5c653` 是 dev 从 main 分叉前最后一个 main 提交（PR #721 合并的第一父提交）
- main 当前 HEAD：`38fa618ba`（2026-10-27，PR #730）
- 统计时间：2026-10-28

## 判定方法

| 状态 | 判定依据 |
| --- | --- |
| 已合入（祖先） | `git merge-base --is-ancestor <c> dev` 为真 |
| 已合入（等效） | `git patch-id --stable` 与 dev 中某个提交一致 |
| 已合入（合并等效） | 合并提交相对第一父的增量内容已全部存在于 dev |
| 部分合入 | 提交内部分文件/功能存在于 dev，其余缺失 |
| 未合入 | 提交的改动在 dev 中完全不存在 |

复现命令：

```bash
git log --oneline --first-parent b4e5c653972bb0966b24dfddcde23b888bda6ebc..main
git cherry -v dev main
git merge-tree --write-tree --name-only dev main   # 整体合并的冲突预览
```

## 提交清单

共 23 个提交（含 7 个合并提交）：17 个已合入，0 个部分合入，6 个未合入。

| # | main 提交 | 类型 | 日期 | 作者 | 标题 | 状态 | dev 对应 / 说明 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `ad49db3de` | commit | 10-25 | BilityLeu | fix(workspace): restore startup focus and unify window closing | 已合入 | dev 历史祖先，随 PR #721 合入 |
| 2 | `142150042` | commit | 10-25 | BilityLeu | fix(palette): prioritize visible name matches across sections | 已合入 | dev 历史祖先，随 PR #722 合入 |
| 3 | `e89621051` | commit | 10-25 | AnalyseDeCircuit | fix(workspace): recover failed fullscreen hides and explain locked quit | 已合入 | dev 历史祖先 |
| 4 | `9abf2769d` | commit | 10-25 | AnalyseDeCircuit | fix(palette): preserve source search and shaped text highlighting | 已合入 | dev 历史祖先 |
| 5 | `3bd74cb17` | merge | 10-25 | LaTerrasse | Merge PR #721 | 已合入 | dev 历史祖先 |
| 6 | `f2593f3bd` | merge | 10-25 | LaTerrasse | Merge PR #722 | 已合入 | dev 历史祖先，dev 分叉点 |
| 7 | `80d8369b1` | commit | 10-26 | livy | fix(cloud-sync): use virtual-hosted S3 endpoints for Alibaba OSS | 已合入 | cherry-pick 为 `ba7cf7734`，patch-id 一致，文件内容完好 |
| 8 | `428cf36a6` | commit | 10-26 | livy | feat(ide): preview Markdown files by default | 已合入 | cherry-pick 为 `af49b1bbf`，patch-id 一致 |
| 9 | `ce5761350` | commit | 10-26 | jaime.su | feat(sidebar): group active sessions by host and user | 未合入 | 无任何痕迹（`Host` 视图模式、`session_group_expanded`、`switch_host` 键、`UserRound` 图标均不存在） |
| 10 | `ba1b46881` | merge | 10-26 | LaTerrasse | Merge PR #725（云同步 OSS） | 已合入 | 合并增量即 #7 的改动，内容等价 |
| 11 | `7299a9838` | commit | 10-26 | AnalyseDeCircuit | perf(terminal): compact scrollback with text runs and shared attributes | 已合入 | cherry-pick 为 `f7673e0ce`，patch-id 一致，27 个文件全部完好 |
| 12 | `20da6d5cf` | commit | 10-26 | AnalyseDeCircuit | fix(sidebar): distinguish grouped SSH ports and jump routes | 未合入 | 依赖 #9，dev 无分组视图代码 |
| 13 | `e479623a1` | merge | 10-26 | LaTerrasse | Merge PR #727（host/user 视图） | 未合入 | 合并增量即 #9 + #12 |
| 14 | `978b70fc3` | commit | 10-26 | AnalyseDeCircuit | ci: deduplicate Linux checks and skip heavy draft and docs jobs | 已合入 | cherry-pick 为 `886ed5330`，patch-id 一致 |
| 15 | `c05a8f665` | commit | 10-26 | AnalyseDeCircuit | ci: avoid repeated platform checks on push | 已合入 | cherry-pick 为 `07ee7e0c5`，patch-id 一致 |
| 16 | `8631ffaaa` | commit | 10-27 | jaime.su | feat(sftp): add sidebar tree view, terminal sync, drop uploads, and path bookmarks | 未合入 | 73 个文件全部未合入（`sftp/tree.rs`、`sftp/bookmarks.rs` 不存在，`open_sftp_on_connect`、外部文件拖放、终端目录同步均缺失） |
| 17 | `a04862646` | commit | 10-27 | jaime.su | fix(ssh): run ProxyCommand helpers without a Windows console window | 已合入 | cherry-pick 为 `d74d9ceb7`，patch-id 一致 |
| 18 | `5097dfdfd` | commit | 10-27 | AnalyseDeCircuit | fix(sftp): guard tree cycles and route actions to safe targets | 未合入 | 依赖 #16；dev 中 `sftp/tree.rs` 不存在 |
| 19 | `f0c9e3975` | commit | 10-27 | AnalyseDeCircuit | fix(ide): support selecting and copying Markdown preview text | 已合入 | cherry-pick 为 `726a29de4`，patch-id 一致 |
| 20 | `e1aae466c` | commit | 10-27 | AnalyseDeCircuit | test(ssh): read complete ProxyCommand pipe response | 已合入 | cherry-pick 为 `d38ea0368`，patch-id 一致 |
| 21 | `48f632556` | merge | 10-27 | LaTerrasse | Merge PR #726（Markdown 预览） | 已合入 | 合并增量即 #8 + #19，内容等价 |
| 22 | `abeb0d29a` | merge | 10-27 | LaTerrasse | Merge PR #729（SFTP 树视图） | 未合入 | 合并增量即 #16 + #18 |
| 23 | `38fa618ba` | merge | 10-27 | LaTerrasse | Merge PR #730（ProxyCommand） | 已合入 | 合并增量即 #17 + #20，内容等价 |

## 未合入提交

| main 提交 | 内容 | 冲突文件（cherry-pick 到 dev 时） | 合入前提 |
| --- | --- | --- | --- |
| `ce5761350` | 活动会话按 host/user 分组的第三种视图 | `workspace/sidebar.rs`、`workspace/sidebar/region.rs`、`workspace/sidebar/sessions.rs` | 无。dev 已重构 sidebar（移除左边栏、活动会话移至右侧栏），需人工改写而非直接套用 |
| `20da6d5cf` | 分组视图内区分 SSH 端口与跳板路由 | `workspace/sidebar/sessions.rs` | 先合 `ce5761350` |
| `8631ffaaa` | SFTP 树视图、终端目录同步、拖放上传、路径书签、连接时自动开 SFTP | 实测仅 `workspace/sidebar/sessions.rs` | 无。剩余 72 个文件（含 5 个新增文件）可自动合入 |
| `5097dfdfd` | 树循环防护与安全目标路由 | `sftp/actions/menus_conflicts.rs`、`sftp/actions/navigation.rs`、`sftp/file_list.rs`、`sftp/runtime.rs`、`sftp/tree.rs`（dev 侧删除） | 先合 `8631ffaaa` |

整体 `git merge main` 的冲突面只有 3 个文件，全部由 #9（host/user 视图）与 dev 的 sidebar 重构重叠造成：

```
CONFLICT crates/oxideterm-gpui-app/src/workspace/sidebar.rs
CONFLICT crates/oxideterm-gpui-app/src/workspace/sidebar/region.rs
CONFLICT crates/oxideterm-gpui-app/src/workspace/sidebar/sessions.rs
```

SFTP 相关提交（#16、#18）在整体合并中不产生冲突，说明 dev 的定制改动没有侵入 SFTP 模块；冲突集中在 session/sidebar 展示层。

## 后续维护

1. `git fetch origin main` 后执行 `git log --oneline <上次记录的 main HEAD>..main`，把新提交追加到清单。
2. 对每个新提交执行 `git cherry -v dev main`，`-` 开头的表示按 patch-id 已被 dev 收录。
3. 未合入的提交合入后，在状态列改为已合入，并填写 dev 侧提交号。
4. 每轮同步后更新顶部「main 当前 HEAD」与「统计时间」。
