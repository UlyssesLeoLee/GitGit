# gitgit CLI — clap derive 命令定义

完整 CLI 命令定义，参考实现见 `crates/gitgit-cli/src/main.rs`。

详细设计：见 `docs/design/detailed-design/08-api-handlers.md` §8.9 (CLI handler) + 任务 105 (生产部署) / 140 (会议报告)。

## 命令树

```
gitgit
├── server
│   ├── start [--daemon]
│   ├── status
│   ├── stop [--force]
│   ├── restart
│   └── logs [-f, --tail N]
├── admin
│   ├── kek-rotate [--dry-run]
│   ├── audit [--actor X] [--action X] [--limit N]
│   └── user
│       ├── disable <id> --reason
│       ├── enable <id>
│       └── reset-mfa <id>
├── app
│   ├── list
│   ├── install --manifest FILE [--skip-verify]
│   ├── uninstall <id> [--force]
│   ├── upgrade <id> [--strategy rolling|blue-green|canary]
│   ├── status <id>
│   ├── check-updates
│   ├── enable <id>
│   └── disable <id>
├── user
│   ├── whoami
│   ├── login --user X [--password-stdin]
│   ├── logout
│   ├── mfa-enroll
│   └── webauthn-enroll
├── graph
│   ├── get <id>
│   ├── create --type T --data JSON
│   ├── delete <id> [--cascade]
│   ├── traverse <start> [--direction out|in|both] [--max-depth 3]
│   └── events [--actor X] [--type X] [--from Y] [--to Y] [--limit N]
├── agent
│   ├── list
│   ├── run <id> --input JSON [--wait]
│   ├── get <run_id>
│   ├── approve <run_id> [--comment X]
│   ├── reject <run_id> --reason
│   └── stream <run_id>
├── backup
│   ├── full --output DIR
│   ├── restore --point-in-time RFC3339
│   ├── list
│   └── verify <backup_id>
├── migrate
│   ├── up [--to N]
│   ├── down <steps>
│   ├── status
│   ├── redo <version>
│   └── new <name>
├── doctor
│   ├── all
│   ├── db
│   ├── git
│   ├── ai
│   └── network
└── config
    ├── show
    ├── validate
    └── list
```

## 全局参数

| 参数 | 简写 | 环境变量 | 说明 |
|---|---|---|---|
| `--config` | `-c` | `GITGIT_CONFIG` | 配置文件路径 |
| `--api-url` | | `GITGIT_API_URL` | API 端点 |
| `--token` | | `GITGIT_TOKEN` | Bearer token |
| `--output` | `-o` | | human / json / yaml |
| `--verbose` | `-v` | | 详细日志 (可叠加) |

## 编译 / 安装

```bash
# 编译
cargo build --release -p gitgit-cli

# 安装到 PATH
cp target/release/gitgit /usr/local/bin/

# 验证
gitgit --version
gitgit doctor all
```
