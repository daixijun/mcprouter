use tauri::ipc::InvokeError;

#[derive(thiserror::Error, Debug)]
pub enum McpError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Service error: {0}")]
    ServiceError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Service already exists: {0}")]
    ServiceAlreadyExists(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("MCP protocol error: {0}")]
    ProtocolError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Permission denied: {0}")]
    PermissionError(String),

    #[error("Database connection failed: {0}")]
    DatabaseConnectionError(String),

    #[error("Database query failed: {0}")]
    DatabaseQueryError(String),

    #[error("Database transaction failed: {0}")]
    DatabaseTransactionError(String),

    #[error("Database migration failed: {0}")]
    DatabaseMigrationError(String),

    #[error("Database initialization failed: {0}")]
    DatabaseInitializationError(String),

    #[error("Resource not found: {0}")]
    NotFoundError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Tool error: {0}")]
    ToolError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Marketplace error: {0}")]
    MarketplaceError(String),

    #[error("Process error: {0}")]
    ProcessError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

// Convert SQLx errors to specific database errors
impl From<sqlx::Error> for McpError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(db_err) => {
                McpError::DatabaseQueryError(format!("Database error: {}", db_err.message()))
            }
            sqlx::Error::PoolTimedOut => {
                McpError::DatabaseConnectionError("Database connection pool timeout".to_string())
            }
            sqlx::Error::PoolClosed => {
                McpError::DatabaseConnectionError("Database connection pool closed".to_string())
            }
            sqlx::Error::RowNotFound => McpError::NotFoundError("Record not found".to_string()),
            sqlx::Error::ColumnDecode { .. } => {
                McpError::DatabaseQueryError(format!("Column decode error: {}", err))
            }
            sqlx::Error::ColumnIndexOutOfBounds { .. } => {
                McpError::DatabaseQueryError(format!("Column index out of bounds: {}", err))
            }
            _ => McpError::DatabaseQueryError(format!("Database error: {}", err)),
        }
    }
}

impl From<McpError> for InvokeError {
    fn from(error: McpError) -> Self {
        InvokeError::from(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, McpError>;
