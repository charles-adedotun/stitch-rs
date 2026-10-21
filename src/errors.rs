//! Domain errors and error taxonomy for the Stitch client.

/// Categorized Stitch client errors.
#[derive(Debug, thiserror::Error)]
pub enum StitchError {
    /// Rate limit or quota exhaustion (HTTP 429).
    #[error("Rate limited{}: {message}", format_tool(.tool))]
    RateLimited {
        message: String,
        tool: Option<String>,
    },

    /// Requested resource was not found (HTTP 404).
    #[error("Resource not found{}: {message}", format_tool(.tool))]
    NotFound {
        message: String,
        tool: Option<String>,
    },

    /// Authentication failure or missing credentials (HTTP 401).
    #[error("Authentication failed{}: {message}", format_tool(.tool))]
    AuthFailed {
        message: String,
        tool: Option<String>,
    },

    /// Insufficient permissions (HTTP 403).
    #[error("Permission denied{}: {message}", format_tool(.tool))]
    PermissionDenied {
        message: String,
        tool: Option<String>,
    },

    /// Server-side JSON-RPC protocol error.
    #[error("JSON-RPC error (code {code}): {message}")]
    Rpc {
        code: i64,
        message: String,
        data: Option<serde_json::Value>,
    },

    /// HTTP response error with status code.
    #[error("HTTP error ({status}): {message}")]
    Http { status: u16, message: String },

    /// Underlying network transport failure.
    #[error("Network error: {0}")]
    Network(String),

    /// Request timeout exceeded.
    #[error("Timeout error: {0}")]
    Timeout(String),

    /// JSON serialization or deserialization failure.
    #[error("JSON error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Local file or filesystem I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// General unclassified Stitch error.
    #[error("Stitch error: {0}")]
    General(String),
}

fn format_tool(tool: &Option<String>) -> String {
    match tool {
        Some(t) => format!(" [{t}]"),
        None => String::new(),
    }
}

impl StitchError {
    /// Returns true if the operation may succeed upon retry.
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::RateLimited { .. } | Self::Timeout(_) => true,
            Self::Http { status, .. } if *status == 429 || *status >= 500 => true,
            _ => false,
        }
    }

    /// Associated HTTP status code if known.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::RateLimited { .. } => Some(429),
            Self::NotFound { .. } => Some(404),
            Self::AuthFailed { .. } => Some(401),
            Self::PermissionDenied { .. } => Some(403),
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for StitchError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout(err.to_string())
        } else if let Some(status) = err.status() {
            Self::Http {
                status: status.as_u16(),
                message: err.to_string(),
            }
        } else {
            Self::Network(err.to_string())
        }
    }
}
