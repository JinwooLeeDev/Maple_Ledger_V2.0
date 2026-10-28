use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Url;
use serde::Deserialize;

use super::{read_response, transport_error, ApiError, NexonClient};

#[derive(Deserialize)]
pub struct BasicInfo {
    pub world_name: String,
    pub character_name: Option<String>,
    pub character_class: Option<String>,
    pub character_level: Option<u32>,
    pub character_image: Option<String>,
}

impl NexonClient {
    pub async fn character_basic(&self, key: &str, ocid: &str) -> Result<BasicInfo, ApiError> {
        let mut url = Url::parse("https://open.api.nexon.com/maplestory/v1/character/basic")
            .map_err(|_| ApiError::new("client_unavailable"))?;
        url.query_pairs_mut().append_pair("ocid", ocid);
        request_count(false);
        parse_basic(&self.request(url.as_str(), key).await?)
    }

    pub async fn character_image(&self, url: &str) -> Result<String, ApiError> {
        let url = trusted_image_url(url)?;
        request_count(true);
        // Public image request: never attach the API key or reuse auth headers.
        let response = self.client.get(url).send().await.map_err(transport_error)?;
        image_data_url(&read_response(response).await?)
    }
}

// Opt-in aggregate development diagnostics, never URLs, identifiers or headers.
fn request_count(image: bool) {
    #[cfg(debug_assertions)]
    if std::env::var_os("MAPLE_LEDGER_REQUEST_COUNTS").is_some() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static BASIC: AtomicUsize = AtomicUsize::new(0);
        static IMAGE: AtomicUsize = AtomicUsize::new(0);
        if image {
            IMAGE.fetch_add(1, Ordering::Relaxed);
        } else {
            BASIC.fetch_add(1, Ordering::Relaxed);
        }
        eprintln!(
            "NEXON request counts: basic={}, image={}",
            BASIC.load(Ordering::Relaxed),
            IMAGE.load(Ordering::Relaxed)
        );
    }
    #[cfg(not(debug_assertions))]
    let _ = image;
}

pub fn parse_basic(body: &[u8]) -> Result<BasicInfo, ApiError> {
    let info: BasicInfo =
        serde_json::from_slice(body).map_err(|_| ApiError::new("response_parse"))?;
    if info.world_name.trim().is_empty() {
        return Err(ApiError::new("response_parse"));
    }
    Ok(info)
}

fn trusted_image_url(value: &str) -> Result<Url, ApiError> {
    let url = Url::parse(value).map_err(|_| ApiError::new("image_unavailable"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("open.api.nexon.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !url.path().starts_with("/static/maplestory/character/look/")
    {
        return Err(ApiError::new("image_unavailable"));
    }
    Ok(url)
}

fn image_data_url(body: &[u8]) -> Result<String, ApiError> {
    if body.len() > 1024 * 1024 {
        return Err(ApiError::new("image_unavailable"));
    }
    let mime = if body.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if body.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else if body.starts_with(b"GIF87a") || body.starts_with(b"GIF89a") {
        "image/gif"
    } else if body.starts_with(b"RIFF") && body.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        return Err(ApiError::new("image_unavailable"));
    };
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(body)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_schema_missing_fields_and_additions_are_safe() {
        assert!(parse_basic(br#"{"world_name":"w","character_name":"c","character_class":"job","character_level":260,"character_image":"https://open.api.nexon.com/static/maplestory/character/look/example","future":true}"#).is_ok());
        let sparse = parse_basic(br#"{"world_name":"w","character_image":null}"#).unwrap();
        assert!(sparse.character_name.is_none() && sparse.character_image.is_none());
        for body in [
            r#"{}"#,
            r#"{"world_name":null}"#,
            r#"{"world_name":""}"#,
            r#"{"world_name":"w","character_level":"260"}"#,
        ] {
            assert!(parse_basic(body.as_bytes()).is_err());
        }
    }

    #[test]
    fn image_origin_and_binary_type_are_restricted() {
        assert!(trusted_image_url(
            "https://open.api.nexon.com/static/maplestory/character/look/example"
        )
        .is_ok());
        for url in [
            "http://open.api.nexon.com/static/maplestory/character/look/x",
            "https://evil.example/static/maplestory/character/look/x",
            "https://open.api.nexon.com.evil.example/static/maplestory/character/look/x",
            "https://user@open.api.nexon.com/static/maplestory/character/look/x",
            "https://open.api.nexon.com:8443/static/maplestory/character/look/x",
            "https://open.api.nexon.com/private",
        ] {
            assert!(trusted_image_url(url).is_err());
        }
        assert!(image_data_url(b"<svg>untrusted</svg>").is_err());
        assert!(image_data_url(b"\x89PNG\r\n\x1a\nsynthetic")
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert!(image_data_url(&vec![0; 1024 * 1024 + 1]).is_err());
    }
}
