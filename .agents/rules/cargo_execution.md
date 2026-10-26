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
  - 触发大范围依赖重新编译的 `cargo test`
  - 修改或引入可能导致 Cargo 重新计算全局构建指纹（Fingerprint）的配置（例如修改 `config.toml` 中的 `linker`、`rustflags` 等全局编译选项）
- AI 进行代码修改后的编译与类型校验时，**仅允许执行快速检查命令**：
  ```bash
  cargo check -p oxideterm-gpui-app
  ```
- 任何完整的二进制构建、客户端运行启动、全量单元测试或正式打包，必须交由用户在终端自行掌控并执行。
