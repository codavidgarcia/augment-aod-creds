use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Scraping error: {0}")]
    Scraping(String),
    
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Notification error: {0}")]
    Notification(String),
    
    #[error("Analytics error: {0}")]
    Analytics(String),
    
    #[error("Invalid token format")]
    InvalidToken,
    
    #[error("Network timeout")]
    Timeout,
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Auth error: {0}")]
    Auth(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl From<AppError> for tauri::Error {
    fn from(err: AppError) -> Self {
        tauri::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
    }
}
