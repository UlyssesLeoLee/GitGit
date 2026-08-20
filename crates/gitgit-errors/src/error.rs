//! 标准化错误类型 (thiserror)
//! 详细设计：§11-error-handling.md

use serde::Serialize;
use std::fmt;

/// 应用层统一错误
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // ============ 通用 (1xx) ============
    #[error("internal error: {0}")]
    Internal(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("not found: {kind} {id}")]
    NotFound { kind: &'static str, id: String },

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("validation failed: {0}")]
    Validation(String),

    // ============ 认证 / 授权 (2xx) ============
    #[error("unauthorized: {0}")]
    Unauthorized(&'static str),

    #[error("forbidden: {action} on {resource_type}/{resource_id}")]
    Forbidden {
        action: String,
        resource_type: String,
        resource_id: String,
    },

    #[error("token expired")]
    TokenExpired,

    #[error("token invalid: {0}")]
    TokenInvalid(String),

    #[error("mfa required")]
    MfaRequired,

    #[error("mfa invalid")]
    MfaInvalid,

    // ============ 资源 (3xx) ============
    #[error("rate limit exceeded (retry after {retry_after_sec}s)")]
    RateLimit { retry_after_sec: u64 },

    #[error("quota exceeded: {resource}")]
    QuotaExceeded { resource: String },

    #[error("conflict: {0}")]
    Conflict(String),

    // ============ 数据库 (4xx) ============
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration failed: {0}")]
    Migration(String),

    #[error("transaction conflict")]
    TransactionConflict,

    // ============ Git 协议 (5xx) ============
    #[error("git error: {0}")]
    Git(String),

    #[error("git object not found: {0}")]
    GitObjectNotFound(String),

    #[error("git ref update rejected: {0}")]
    GitRefRejected(String),

    #[error("hook failed: {0}")]
    HookFailed(String),

    // ============ AI (6xx) ============
    #[error("ai provider error: {provider}: {message}")]
    AiProvider { provider: String, message: String },

    #[error("ai rate limit")]
    AiRateLimit,

    #[error("prompt injection detected: {0}")]
    PromptInjectionDetected(String),

    #[error("mcp tool not found: {0}")]
    McpToolNotFound(String),

    #[error("mcp tool denied: {0}")]
    McpToolDenied(String),

    // ============ App / Plugin (7xx) ============
    #[error("app not found: {0}")]
    AppNotFound(String),

    #[error("app install failed: {0}")]
    AppInstallFailed(String),

    #[error("app upgrade failed: {0}")]
    AppUpgradeFailed(String),

    #[error("app permission denied: {app_id} tries to {action} on {resource}")]
    AppPermissionDenied {
        app_id: String,
        action: String,
        resource: String,
    },

    #[error("app sandbox violation: {0}")]
    AppSandboxViolation(String),

    // ============ 事件总线 (8xx) ============
    #[error("event publish failed: {0}")]
    EventPublishFailed(String),

    #[error("event delivery failed: {subscriber} after {retries} retries")]
    EventDeliveryFailed { subscriber: String, retries: u32 },

    #[error("event DLQ overflow: {0}")]
    EventDlqOverflow(String),

    // ============ Secrets (9xx) ============
    #[error("secret not found: {0}")]
    SecretNotFound(String),

    #[error("secret decrypt failed")]
    SecretDecryptFailed,

    #[error("KEK rotation in progress")]
    KekRotating,

    // ============ 外部 (10xx) ============
    #[error("external service unavailable: {service}")]
    ExternalUnavailable { service: String },

    #[error("upstream timeout: {service} after {timeout_sec}s")]
    UpstreamTimeout { service: String, timeout_sec: u64 },
}

/// 错误代码 (HTTP-friendly, 5 字符)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // 1xx
    InternalError,
    ConfigError,
    NotFound,
    InvalidRequest,
    ValidationFailed,
    // 2xx
    Unauthorized,
    Forbidden,
    TokenExpired,
    TokenInvalid,
    MfaRequired,
    MfaInvalid,
    // 3xx
    RateLimit,
    QuotaExceeded,
    Conflict,
    // 4xx
    DatabaseError,
    MigrationFailed,
    TransactionConflict,
    // 5xx
    GitError,
    GitObjectNotFound,
    GitRefRejected,
    HookFailed,
    // 6xx
    AiProvider,
    AiRateLimit,
    PromptInjectionDetected,
    McpToolNotFound,
    McpToolDenied,
    // 7xx
    AppNotFound,
    AppInstallFailed,
    AppUpgradeFailed,
    AppPermissionDenied,
    AppSandboxViolation,
    // 8xx
    EventPublishFailed,
    EventDeliveryFailed,
    EventDlqOverflow,
    // 9xx
    SecretNotFound,
    SecretDecryptFailed,
    KekRotating,
    // 10xx
    ExternalUnavailable,
    UpstreamTimeout,
}

impl AppError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Internal(_) => ErrorCode::InternalError,
            Self::Config(_) => ErrorCode::ConfigError,
            Self::NotFound { .. } => ErrorCode::NotFound,
            Self::InvalidRequest(_) => ErrorCode::InvalidRequest,
            Self::Validation(_) => ErrorCode::ValidationFailed,
            Self::Unauthorized(_) => ErrorCode::Unauthorized,
            Self::Forbidden { .. } => ErrorCode::Forbidden,
            Self::TokenExpired => ErrorCode::TokenExpired,
            Self::TokenInvalid(_) => ErrorCode::TokenInvalid,
            Self::MfaRequired => ErrorCode::MfaRequired,
            Self::MfaInvalid => ErrorCode::MfaInvalid,
            Self::RateLimit { .. } => ErrorCode::RateLimit,
            Self::QuotaExceeded { .. } => ErrorCode::QuotaExceeded,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::Database(_) => ErrorCode::DatabaseError,
            Self::Migration(_) => ErrorCode::MigrationFailed,
            Self::TransactionConflict => ErrorCode::TransactionConflict,
            Self::Git(_) => ErrorCode::GitError,
            Self::GitObjectNotFound(_) => ErrorCode::GitObjectNotFound,
            Self::GitRefRejected(_) => ErrorCode::GitRefRejected,
            Self::HookFailed(_) => ErrorCode::HookFailed,
            Self::AiProvider { .. } => ErrorCode::AiProvider,
            Self::AiRateLimit => ErrorCode::AiRateLimit,
            Self::PromptInjectionDetected(_) => ErrorCode::PromptInjectionDetected,
            Self::McpToolNotFound(_) => ErrorCode::McpToolNotFound,
            Self::McpToolDenied(_) => ErrorCode::McpToolDenied,
            Self::AppNotFound(_) => ErrorCode::AppNotFound,
            Self::AppInstallFailed(_) => ErrorCode::AppInstallFailed,
            Self::AppUpgradeFailed(_) => ErrorCode::AppUpgradeFailed,
            Self::AppPermissionDenied { .. } => ErrorCode::AppPermissionDenied,
            Self::AppSandboxViolation(_) => ErrorCode::AppSandboxViolation,
            Self::EventPublishFailed(_) => ErrorCode::EventPublishFailed,
            Self::EventDeliveryFailed { .. } => ErrorCode::EventDeliveryFailed,
            Self::EventDlqOverflow(_) => ErrorCode::EventDlqOverflow,
            Self::SecretNotFound(_) => ErrorCode::SecretNotFound,
            Self::SecretDecryptFailed => ErrorCode::SecretDecryptFailed,
            Self::KekRotating => ErrorCode::KekRotating,
            Self::ExternalUnavailable { .. } => ErrorCode::ExternalUnavailable,
            Self::UpstreamTimeout { .. } => ErrorCode::UpstreamTimeout,
        }
    }

    /// HTTP 状态码映射
    pub fn http_status(&self) -> u16 {
        use ErrorCode::*;
        match self.code() {
            NotFound | GitObjectNotFound | AppNotFound | SecretNotFound | McpToolNotFound => 404,
            Unauthorized | TokenExpired | TokenInvalid => 401,
            Forbidden | MfaRequired | MfaInvalid | AppPermissionDenied | McpToolDenied | AppSandboxViolation => 403,
            Validation | InvalidRequest => 400,
            Conflict | TransactionConflict | GitRefRejected | HookFailed | KekRotating => 409,
            RateLimit | AiRateLimit => 429,
            QuotaExceeded => 429,
            PromptInjectionDetected => 422,
            EventDlqOverflow => 503,
            ExternalUnavailable | UpstreamTimeout | AiProvider | DatabaseError | MigrationFailed
            | GitError | EventPublishFailed | EventDeliveryFailed | SecretDecryptFailed
            | InternalError | ConfigError => 500,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 3)?;
        st.serialize_field("code", &self.code())?;
        st.serialize_field("message", &self.to_string())?;
        // 不暴露内部 details (避免信息泄露)
        st.skip_field("details")?;
        st.end()
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Validation(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(format!("io: {e}"))
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
