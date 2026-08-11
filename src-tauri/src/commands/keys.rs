use serde::Serialize;
use tauri::State;

use crate::error::Result;
use crate::security::keys;
use crate::state::AppState;

#[tauri::command]
pub fn set_api_key(key: String) -> Result<()> {
    keys::set_api_key(&key)
}

#[tauri::command]
pub fn has_api_key() -> bool {
    keys::has_api_key()
}

#[tauri::command]
pub fn delete_api_key() -> Result<()> {
    keys::delete_api_key()
}

#[derive(Serialize)]
pub struct TestApiKeyResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[tauri::command]
pub async fn test_api_key(state: State<'_, AppState>) -> Result<TestApiKeyResult> {
    let key = match keys::get_api_key() {
        Ok(k) => k,
        Err(_) => {
            return Ok(TestApiKeyResult {
                ok: false,
                error: Some("no API key saved".into()),
            })
        }
    };

    match state
        .http
        .get("https://api.openai.com/v1/models")
        .bearer_auth(&key)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => Ok(TestApiKeyResult { ok: true, error: None }),
        Ok(resp) if resp.status() == reqwest::StatusCode::UNAUTHORIZED => Ok(TestApiKeyResult {
            ok: false,
            error: Some("OpenAI rejected this key (401 Unauthorized)".into()),
        }),
        Ok(resp) => Ok(TestApiKeyResult {
            ok: false,
            error: Some(format!("unexpected response: HTTP {}", resp.status())),
        }),
        Err(e) => Ok(TestApiKeyResult {
            ok: false,
            error: Some(format!("network error: {e}")),
        }),
    }
}
