---
trigger: always_on
glob:
description:
---

# Cargo Execution Constraints

- AI 严禁擅自执行可能打乱编译缓存、触发全量重新构建或显著增加本地编译时长的 Cargo 命令。
- 严禁 AI 执行的操作包括但不限于：
  - `cargo build`（无论是 debug 还是 --release）
  - `cargo run`
  - `cargo clean`
  - `cargo test`、`cargo check --tests`、`cargo check --all-targets`、`cargo nextest` 等任何编译或运行测试目标（`#[cfg(test)]` 代码）的操作
  - 修改或引入可能导致 Cargo 重新计算全局构建指纹（Fingerprint）的配置（例如修改 `config.toml` 中的 `linker`、`rustflags` 等全局编译选项）
- AI 进行代码修改后的编译与类型校验时，**仅允许执行快速检查命令**，且不带测试目标：
  ```bash
  cargo check -p oxideterm-gpui-app
  ```
- 禁止用独立 `CARGO_TARGET_DIR`、临时 worktree 或临时克隆绕开上述限制。判断补丁或分支能否合入时，只用 `git` 与源码级静态检查（`git apply --3way` 试摘、`git patch-id` 比对、符号 `grep`），不以编译结果代替。
- 不触发 Rust 编译的静态检查不受限制，例如 `python scripts/quality/audit_i18n.py`。
- 任何完整的二进制构建、客户端运行启动、单元测试或正式打包，必须交由用户在终端自行掌控并执行。
