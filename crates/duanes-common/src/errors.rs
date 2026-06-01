use thiserror::Error;

#[derive(Debug, Error)]
pub enum DuanesError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Governance policy violated: {0}")]
    GovernanceViolation(String),

    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("Rate limit exceeded for user: {0}")]
    RateLimitExceeded(String),

    #[error("Budget exceeded for user: {0}")]
    BudgetExceeded(String),

    #[error("Model not allowed: {0}")]
    ModelNotAllowed(String),

    #[error("Local inference error: {0}")]
    LocalInference(String),

    #[error("Request timeout after {0}ms")]
    Timeout(u64),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Collector sync error: {0}")]
    CollectorSync(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for DuanesError {
    fn from(e: serde_json::Error) -> Self {
        DuanesError::Serialization(e.to_string())
    }
}

impl From<toml::de::Error> for DuanesError {
    fn from(e: toml::de::Error) -> Self {
        DuanesError::Config(e.to_string())
    }
}

pub type DuanesResult<T> = Result<T, DuanesError>;
