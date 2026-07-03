use crate::{CoreError, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Debug)]
pub enum PollResult {
    Pending,
    SlowDown,
    Token(Token),
}

#[derive(Debug, Deserialize)]
struct TokenOrError {
    error: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

pub fn request_device_code(base: &str, client_id: &str) -> Result<DeviceCode> {
    let resp: DeviceCode = ureq::post(&format!("{base}/login/device/code"))
        .set("Accept", "application/json")
        .send_form(&[("client_id", client_id)])
        .map_err(|e| CoreError::Http(e.to_string()))?
        .into_json()
        .map_err(|e| CoreError::Http(e.to_string()))?;
    Ok(resp)
}

pub fn poll_once(base: &str, client_id: &str, device_code: &str) -> Result<PollResult> {
    let resp: TokenOrError = ureq::post(&format!("{base}/login/oauth/access_token"))
        .set("Accept", "application/json")
        .send_form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .map_err(|e| CoreError::Http(e.to_string()))?
        .into_json()
        .map_err(|e| CoreError::Http(e.to_string()))?;
    match resp.error.as_deref() {
        Some("authorization_pending") => Ok(PollResult::Pending),
        Some("slow_down") => Ok(PollResult::SlowDown),
        Some(e) => Err(CoreError::Http(format!("device flow: {e}"))),
        None => Ok(PollResult::Token(Token {
            access_token: resp
                .access_token
                .ok_or_else(|| CoreError::Http("missing access_token".into()))?,
            refresh_token: resp.refresh_token,
            expires_in: resp.expires_in,
        })),
    }
}

pub fn refresh(base: &str, client_id: &str, refresh_token: &str) -> Result<Token> {
    let resp: TokenOrError = ureq::post(&format!("{base}/login/oauth/access_token"))
        .set("Accept", "application/json")
        .send_form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .map_err(|e| CoreError::Http(e.to_string()))?
        .into_json()
        .map_err(|e| CoreError::Http(e.to_string()))?;
    if let Some(e) = resp.error {
        return Err(CoreError::Http(format!("refresh: {e}")));
    }
    Ok(Token {
        access_token: resp
            .access_token
            .ok_or_else(|| CoreError::Http("missing access_token".into()))?,
        refresh_token: resp.refresh_token,
        expires_in: resp.expires_in,
    })
}
