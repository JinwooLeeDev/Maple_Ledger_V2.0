use std::{collections::HashSet, time::Duration};

use reqwest::{header::HeaderValue, Client, StatusCode};
use serde::{Deserialize, Serialize};
pub mod basic;
pub mod scheduler;

const ENDPOINT: &str = "https://open.api.nexon.com/maplestory/v1/character/list";
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ApiError {
    pub code: &'static str,
    pub retry_after_seconds: Option<u64>,
}

impl ApiError {
    pub fn new(code: &'static str) -> Self {
        Self {
            code,
            retry_after_seconds: None,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct CharacterList {
    pub account_list: Vec<Account>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Account {
    pub account_id: String,
    pub character_list: Vec<Character>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Character {
    pub ocid: String,
    pub world_name: String,
    pub character_name: Option<String>,
    pub character_class: Option<String>,
    pub character_level: Option<u32>,
}

pub struct NexonClient {
    client: Client,
}

impl NexonClient {
    pub fn new() -> Result<Self, ApiError> {
        let client = Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| ApiError::new("client_unavailable"))?;
        Ok(Self { client })
    }

    pub async fn character_list(&self, key: &str) -> Result<CharacterList, ApiError> {
        let body = self.request(ENDPOINT, key).await?;
        parse_character_list(&body)
    }

    async fn request(&self, endpoint: &str, key: &str) -> Result<Vec<u8>, ApiError> {
        let mut header = HeaderValue::from_str(key).map_err(|_| ApiError::new("authentication"))?;
        header.set_sensitive(true);
        let response = self
            .client
            .get(endpoint)
            .header("x-nxopen-api-key", header)
            .send()
            .await
            .map_err(transport_error)?;
        read_response(response).await
    }
}

async fn read_response(mut response: reqwest::Response) -> Result<Vec<u8>, ApiError> {
    let status = response.status();
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if response
        .content_length()
        .is_some_and(|size| size > MAX_BODY_BYTES as u64)
    {
        return Err(if status.is_success() {
            ApiError::new("response_parse")
        } else {
            http_error(status, &[], retry_after)
        });
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(if status.is_success() {
                ApiError::new("response_parse")
            } else {
                http_error(status, &[], retry_after)
            });
        }
        body.extend_from_slice(&chunk);
    }
    if status != StatusCode::OK {
        return Err(http_error(status, &body, retry_after));
    }
    Ok(body)
}

fn transport_error(error: reqwest::Error) -> ApiError {
    ApiError::new(if error.is_timeout() {
        "timeout"
    } else {
        "network"
    })
}

fn http_error(status: StatusCode, body: &[u8], retry_after: Option<u64>) -> ApiError {
    // Only interpret the documented error name; never return message or body.
    let value = serde_json::from_slice::<serde_json::Value>(body).ok();
    let name = value
        .as_ref()
        .and_then(|v| v.pointer("/error/name"))
        .and_then(|v| v.as_str());
    if status == StatusCode::TOO_MANY_REQUESTS || name == Some("OPENAPI00007") {
        return ApiError {
            code: "rate_limited",
            retry_after_seconds: Some(retry_after.unwrap_or(60).max(1)),
        };
    }
    ApiError::new(
        if status == StatusCode::UNAUTHORIZED
            || status == StatusCode::FORBIDDEN
            || name == Some("OPENAPI00005")
        {
            "authentication"
        } else if status.is_server_error() || matches!(name, Some("OPENAPI00010" | "OPENAPI00011"))
        {
            "server"
        } else if name == Some("OPENAPI00009") {
            "data_pending"
        } else {
            "request_rejected"
        },
    )
}

pub fn parse_character_list(body: &[u8]) -> Result<CharacterList, ApiError> {
    let mut result: CharacterList =
        serde_json::from_slice(body).map_err(|_| ApiError::new("response_parse"))?;
    let mut account_ids = HashSet::new();
    let mut ocids = HashSet::new();
    for account in &mut result.account_list {
        if account.account_id.trim().is_empty() || !account_ids.insert(account.account_id.clone()) {
            return Err(ApiError::new("response_parse"));
        }
        for character in &account.character_list {
            if character.ocid.trim().is_empty()
                || character.world_name.trim().is_empty()
                || !ocids.insert(character.ocid.clone())
            {
                return Err(ApiError::new("response_parse"));
            }
        }
        account.character_list.sort_by(|a, b| {
            (&a.world_name, &a.character_name, &a.ocid).cmp(&(
                &b.world_name,
                &b.character_name,
                &b.ocid,
            ))
        });
    }
    result
        .account_list
        .sort_by(|a, b| a.account_id.cmp(&b.account_id));
    Ok(result)
}

#[cfg(test)]
mod tests;
