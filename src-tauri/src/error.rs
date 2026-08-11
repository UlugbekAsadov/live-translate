use serde::ser::SerializeStruct;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[allow(dead_code)] // reserved: pipeline currently reports this via app:error events
    #[error("no audio device: {0}")]
    NoDevice(String),
    #[error("invalid or missing API key")]
    InvalidKey,
    #[error("credential storage error: {0}")]
    Keyring(String),
    #[error("audio error: {0}")]
    Audio(String),
    #[allow(dead_code)] // reserved: network failures are reported via app:error events
    #[error("network error: {0}")]
    Network(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NoDevice(_) => "no_device",
            AppError::InvalidKey => "invalid_key",
            AppError::Keyring(_) => "keyring",
            AppError::Audio(_) => "device_lost",
            AppError::Network(_) => "network",
            AppError::Internal(_) => "internal",
        }
    }
}

// Serialized as `{ code, message }` so the frontend can branch on `code`.
// (`std::result::Result` spelled out: the `Result` alias below takes one param.)
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
