//! CLI 命令定义 (clap derive)
//! 详细设计：§08-api-handlers.md §8.9 + 任务 105 (生产部署) / 140 (会议报告)
//! 关联二进制：gitgit-server / gitgit-admin / gitgit-cli (Cargo workspace 成员)

use clap::{Parser, Subcommand};

/// GitGit 平台 CLI
#[derive(Debug, Parser)]
#[command(name = "gitgit", version, about = "AI-Native Engineering Platform CLI")]
pub struct Cli {
    /// 配置文件路径
    #[arg(long, global = true, env = "GITGIT_CONFIG")]
    pub config: Option<String>,

    /// API endpoint
    #[arg(long, global = true, env = "GITGIT_API_URL", default_value = "http://localhost:3000")]
    pub api_url: String,

    /// Auth token (或 GITGIT_TOKEN 环境变量)
    #[arg(long, global = true, env = "GITGIT_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    /// 输出格式
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,

    /// 详细日志 (-v, -vv, -vvv)
    #[arg(long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Yaml,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 平台进程管理 (启动 / 状态 / 关闭)
    Server(ServerCmd),

    /// Admin 操作 (独立鉴权域)
    Admin(AdminCmd),

    /// App 插件管理
    App(AppCmd),

    /// 用户 / 认证
    User(UserCmd),

    /// 图谱查询
    Graph(GraphCmd),

    /// Agent 运行
    Agent(AgentCmd),

    /// 备份 / 恢复
    Backup(BackupCmd),

    /// 迁移
    Migrate(MigrateCmd),

    /// 健康检查
    #[command(subcommand)]
    Doctor(DoctorCmd),

    /// 配置查看 / 验证
    #[command(subcommand)]
    Config(ConfigCmd),
}

#[derive(Debug, Parser)]
pub struct ServerCmd {
    #[command(subcommand)]
    pub action: ServerAction,
}

#[derive(Debug, Subcommand)]
pub enum ServerAction {
    /// 启动服务 (默认前台)
    Start {
        /// 后台运行
        #[arg(long)]
        daemon: bool,
    },
    /// 查看状态
    Status,
    /// 优雅关闭
    Stop {
        /// 强制立即关闭
        #[arg(long)]
        force: bool,
    },
    /// 重启
    Restart,
    /// 查看日志
    Logs {
        /// 跟踪模式
        #[arg(long, short)]
        follow: bool,
        /// 尾部行数
        #[arg(long, default_value = "100")]
        tail: u32,
    },
}

#[derive(Debug, Parser)]
pub struct AdminCmd {
    #[command(subcommand)]
    pub action: AdminAction,
}

#[derive(Debug, Subcommand)]
pub enum AdminAction {
    /// KEK 轮换
    KekRotate {
        /// 干运行 (仅打印计划)
        #[arg(long)]
        dry_run: bool,
    },
    /// 查看 admin_audit
    Audit {
        #[arg(long)]
        actor: Option<String>,
        #[arg(long)]
        action: Option<String>,
        #[arg(long, default_value = "100")]
        limit: u32,
    },
    /// 启用/禁用用户
    User {
        #[command(subcommand)]
        action: AdminUserAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum AdminUserAction {
    Disable { user_id: String, reason: String },
    Enable { user_id: String },
    ResetMfa { user_id: String },
}

#[derive(Debug, Parser)]
pub struct AppCmd {
    #[command(subcommand)]
    pub action: AppAction,
}

#[derive(Debug, Subcommand)]
pub enum AppAction {
    /// 列出已安装 App
    List,
    /// 安装 App (从 app.yaml)
    Install {
        /// 本地 manifest 文件路径
        #[arg(long)]
        manifest: String,
        /// 跳过签名验证 (仅开发)
        #[arg(long)]
        skip_verify: bool,
    },
    /// 卸载 App
    Uninstall { app_id: String, #[arg(long)] force: bool },
    /// 升级 App
    Upgrade {
        app_id: String,
        #[arg(long, value_enum, default_value_t = UpgradeStrategy::Rolling)]
        strategy: UpgradeStrategy,
    },
    /// 查看 App 状态
    Status { app_id: String },
    /// 列出可用升级
    CheckUpdates,
    /// 启用/禁用
    Enable { app_id: String },
    Disable { app_id: String },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum UpgradeStrategy {
    Rolling,
    BlueGreen,
    Canary,
}

#[derive(Debug, Parser)]
pub struct UserCmd {
    #[command(subcommand)]
    pub action: UserAction,
}

#[derive(Debug, Subcommand)]
pub enum UserAction {
    /// 当前登录用户信息
    Whoami,
    /// 登录
    Login {
        #[arg(long)]
        username: String,
        /// 不在 TTY 时使用 (CI)
        #[arg(long)]
        password_stdin: bool,
    },
    /// 登出
    Logout,
    /// 启用 MFA
    MfaEnroll,
    /// 注册 WebAuthn
    WebauthnEnroll,
}

#[derive(Debug, Parser)]
pub struct GraphCmd {
    #[command(subcommand)]
    pub action: GraphAction,
}

#[derive(Debug, Subcommand)]
pub enum GraphAction {
    /// 查询节点
    Get {
        /// 节点 ID
        id: String,
    },
    /// 创建节点
    Create {
        #[arg(long)]
        node_type: String,
        /// 数据 (JSON 字符串)
        #[arg(long)]
        data: String,
    },
    /// 删除节点
    Delete {
        id: String,
        #[arg(long)]
        cascade: bool,
    },
    /// 图遍历
    Traverse {
        /// 起始节点 ID
        start: String,
        #[arg(long, value_enum, default_value_t = TraverseDirection::Out)]
        direction: TraverseDirection,
        #[arg(long, default_value = "3")]
        max_depth: u32,
    },
    /// 查询事件
    Events {
        #[arg(long)]
        actor: Option<String>,
        #[arg(long)]
        type_: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u32,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum TraverseDirection {
    Out,
    In,
    Both,
}

#[derive(Debug, Parser)]
pub struct AgentCmd {
    #[command(subcommand)]
    pub action: AgentAction,
}

#[derive(Debug, Subcommand)]
pub enum AgentAction {
    /// 列出已注册 Agent
    List,
    /// 启动 Agent 运行
    Run {
        agent_id: String,
        /// 输入 (JSON 字符串)
        #[arg(long)]
        input: String,
        /// 自动等待并打印结果
        #[arg(long)]
        wait: bool,
    },
    /// 查看运行
    Get { run_id: String },
    /// 批准运行
    Approve { run_id: String, #[arg(long)] comment: Option<String> },
    /// 拒绝运行
    Reject { run_id: String, #[arg(long)] reason: String },
    /// 流式获取事件
    Stream { run_id: String },
}

#[derive(Debug, Parser)]
pub struct BackupCmd {
    #[command(subcommand)]
    pub action: BackupAction,
}

#[derive(Debug, Subcommand)]
pub enum BackupAction {
    /// 全量备份
    Full {
        /// 输出目录
        #[arg(long)]
        output: String,
    },
    /// PITR 时间点恢复
    Restore {
        /// 目标时间 (RFC3339)
        #[arg(long)]
        point_in_time: String,
    },
    /// 列出已有备份
    List,
    /// 验证备份可恢复
    Verify { backup_id: String },
}

#[derive(Debug, Parser)]
pub struct MigrateCmd {
    #[command(subcommand)]
    pub action: MigrateAction,
}

#[derive(Debug, Subcommand)]
pub enum MigrateAction {
    /// 应用迁移
    Up {
        /// 应用到第 N 个 (默认全部)
        #[arg(long)]
        to: Option<u32>,
    },
    /// 回滚最后 N 个
    Down { steps: u32 },
    /// 状态
    Status,
    /// 强制重做 (修复)
    Redo { version: u32 },
    /// 创建新 migration 文件
    New {
        /// migration 名 (kebab-case)
        name: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum DoctorCmd {
    /// 完整健康检查
    All,
    /// 仅 DB
    Db,
    /// 仅 Git 存储
    Git,
    /// 仅 AI provider
    Ai,
    /// 网络连通性
    Network,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    /// 打印合并后的有效配置 (隐藏 secret)
    Show,
    /// 验证配置 schema
    Validate,
    /// 列出可配置项
    List,
}
