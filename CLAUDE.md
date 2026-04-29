# rcheat

Linux 进程内存读写 CLI 工具，基于 ptrace 附加目标进程后，读取/修改其变量值。
支持 ELF 解析、DWARF 调试信息匹配、Lua 脚本格式化输出。

## Build

```bash
cargo build                   # debug
cargo build --release         # release
cargo build --target x86_64-unknown-linux-musl --release  # static musl
```

Musl target 已在 `.cargo/config.toml` 配置静态链接。

## Test / Lint

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```

CI 在 `build-test.yml` 中对 x86_64/aarch64/arm 的 gnu 和 musl target 做交叉编译验证。

## Architecture

- `src/main.rs` — 入口，clap CLI 参数定义 (`Args`)
- `src/ctrl.rs` — ptrace 附加/读写内存、解析 `/proc/[pid]/maps`、核心控制逻辑 (`further_parse`)
- `src/elf/elfmgr.rs` — ELF 文件解析，符号表查找
- `src/elf/dwinfo.rs` — DWARF 调试信息匹配（gimli + object）
- `src/qpid.rs` — 按名称查进程 (`ProcessAttr`)
- `src/lua.rs` — mlua 集成，Lua 脚本格式化数据
- `src/fmt_dump.rs` — 输出格式化 (tabled)
- `src/macros.rs` — 工具宏
- `lua/eg.lua` — Lua 脚本示例

## Key Dependencies

| crate | 用途 |
|-------|------|
| nix | ptrace 系统调用 (Unix only) |
| goblin | ELF 解析 |
| gimli + object | DWARF 调试信息 |
| clap (derive) | CLI 参数 |
| mlua (lua54, vendored) | Lua 脚本引擎 |
| owo-colors | 终端彩色输出 |
| tabled (ansi) | 表格输出 |
| symbolic-demangle | C++/Rust 符号 demangle |
| anyhow | 错误处理 |

## Platform

- 主目标: Linux (需要 root 权限运行，因 ptrace)
- CI 也构建 macOS target (aarch64/x86_64 apple-darwin)
- 构建产物打包: .deb (`cargo deb`) 和 .rpm (`cargo generate-rpm`)

## Git Workflow

- 主开发分支: `dev0`，发布分支: `master`
- CI 触发: push 到 `dev*`，PR 到 `master`
- Git commit 使用 Conventional Commits 格式

## build.rs

构建时注入环境变量: `RCHEAT_BUILD_TIME`, `RCHEAT_BUILD_GIT_HASH`, `RCHEAT_GIT_TAG_VERSION`, `RCHEAT_GIT_IS_CLEAN_COMMIT`。

## Conventions

- Rust edition 2021
- `type AnyError = Result<(), anyhow::Error>` 作为统一错误类型
- 不要自动 commit，修改后先展示 diff
- 命令用单行格式（不要反斜杠续行）
