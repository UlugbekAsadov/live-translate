//! OpenAI API key storage via the OS credential store
//! (Windows Credential Manager on Windows, Keychain on macOS for development).
//! The key never touches a config file and is never sent to the frontend.

use keyring::Entry;

use crate::error::{AppError, Result};

const SERVICE: &str = "live-translate";
const USER: &str = "openai_api_key";

fn entry() -> Result<Entry> {
    Entry::new(SERVICE, USER).map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn set_api_key(key: &str) -> Result<()> {
    let key = key.trim();
    if key.len() < 8 || !key.starts_with("sk-") {
        return Err(AppError::Internal(
            "that does not look like an OpenAI API key (expected it to start with sk-)".into(),
        ));
    }
    entry()?
        .set_password(key)
        .map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn get_api_key() -> Result<String> {
    match entry()?.get_password() {
        Ok(k) => Ok(k),
        Err(keyring::Error::NoEntry) => Err(AppError::InvalidKey),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

pub fn has_api_key() -> bool {
    matches!(entry().map(|e| e.get_password()), Ok(Ok(_)))
}

pub fn delete_api_key() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}
