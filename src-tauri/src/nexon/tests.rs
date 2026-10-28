use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

const FIXTURE: &str = r#"{"account_list":[
 {"account_id":"account-b","character_list":[]},
 {"account_id":"account-a","character_list":[
  {"ocid":"test-2","world_name":"월드 나","character_name":"테스트 둘","character_class":"직업 나","character_level":220},
  {"ocid":"test-1","world_name":"월드 가","character_name":"테스트 하나","character_class":"직업 가","character_level":260,"future_field":true}
 ]}],"future_root":true}"#;

#[test]
fn parse_preserves_identity_and_ignores_response_order() {
    let result = parse_character_list(FIXTURE.as_bytes()).unwrap();
    assert_eq!(result.account_list.len(), 2);
    let account = &result.account_list[0];
    assert_eq!(account.account_id, "account-a");
    assert_eq!(account.character_list[0].ocid, "test-1");
    assert_eq!(account.character_list[0].world_name, "월드 가");
    assert_eq!(account.character_list[0].character_level, Some(260));
    assert!(result.account_list[1].character_list.is_empty());
}

#[test]
fn empty_optional_missing_and_invalid_schemas_are_distinct() {
    assert!(parse_character_list(br#"{"account_list":[]}"#)
        .unwrap()
        .account_list
        .is_empty());
    let sparse = br#"{"account_list":[{"account_id":"a","character_list":[{"ocid":"c","world_name":"w","character_name":null}]}]}"#;
    assert!(
        parse_character_list(sparse).unwrap().account_list[0].character_list[0]
            .character_level
            .is_none()
    );
    for invalid in [
        r#"{}"#,
        r#"{"account_list":null}"#,
        r#"{"account_list":[{"account_id":"a"}]}"#,
        r#"{"account_list":[{"account_id":"","character_list":[] }]}"#,
        r#"{"account_list":[{"account_id":"a","character_list":[{"ocid":"c"}]}]}"#,
        r#"{"account_list":[{"account_id":"a","character_list":[{"ocid":"c","world_name":"w","character_level":"260"}]}]}"#,
    ] {
        assert!(parse_character_list(invalid.as_bytes()).is_err());
    }
    let duplicate = FIXTURE.replace("test-2", "test-1");
    assert!(parse_character_list(duplicate.as_bytes()).is_err());
}

#[test]
fn http_errors_are_safe_and_invalid_key_400_is_authentication() {
    let invalid_key = br#"{"error":{"name":"OPENAPI00005","message":"sensitive-server-message"}}"#;
    assert_eq!(
        http_error(StatusCode::BAD_REQUEST, invalid_key, None).code,
        "authentication"
    );
    for (status, expected) in [
        (401, "authentication"),
        (403, "authentication"),
        (429, "rate_limited"),
        (500, "server"),
        (503, "server"),
        (400, "request_rejected"),
    ] {
        let error = http_error(
            StatusCode::from_u16(status).unwrap(),
            b"sensitive-server-message",
            None,
        );
        assert_eq!(error.code, expected);
        assert!(!format!("{error:?}").contains("sensitive-server-message"));
    }
    assert_eq!(
        http_error(StatusCode::TOO_MANY_REQUESTS, &[], Some(123)).retry_after_seconds,
        Some(123)
    );
}

fn server(status: &str, body: &str, delay: Duration) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/list", listener.local_addr().unwrap());
    let response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}", body.len());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = [0; 4096];
        let length = stream.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..length]);
        assert!(request.contains("x-nxopen-api-key: synthetic-header-only"));
        thread::sleep(delay);
        let _ = stream.write_all(response.as_bytes());
    });
    (url, handle)
}

#[test]
fn local_transport_checks_header_timeout_network_and_parse() {
    tauri::async_runtime::block_on(async {
        let client = NexonClient {
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap(),
        };
        let (url, handle) = server("200 OK", FIXTURE, Duration::ZERO);
        let body = client.request(&url, "synthetic-header-only").await.unwrap();
        assert!(parse_character_list(&body).is_ok());
        handle.join().unwrap();
        let (url, handle) = server("200 OK", "not-json", Duration::ZERO);
        let body = client.request(&url, "synthetic-header-only").await.unwrap();
        assert!(parse_character_list(&body).is_err());
        handle.join().unwrap();
        let (url, handle) = server("200 OK", FIXTURE, Duration::from_millis(250));
        let fast = NexonClient {
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_millis(80))
                .build()
                .unwrap(),
        };
        assert_eq!(
            fast.request(&url, "synthetic-header-only")
                .await
                .unwrap_err()
                .code,
            "timeout"
        );
        handle.join().unwrap();
        // A closed Windows port may reach the client timeout before TCP reports
        // refusal. An invalid HTTP status deterministically tests non-timeout I/O.
        let (url, handle) = server("invalid-status", "", Duration::ZERO);
        assert_eq!(
            client
                .request(&url, "synthetic-header-only")
                .await
                .unwrap_err()
                .code,
            "network"
        );
        handle.join().unwrap();
    });
}

#[test]
#[ignore = "Uses one real Nexon request; run explicitly, never in routine tests"]
fn live_character_list_schema() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("secrets/nexon-api-key.txt");
    let key = zeroize::Zeroizing::new(
        std::fs::read_to_string(path).expect("Development key file could not be read"),
    );
    tauri::async_runtime::block_on(async {
        let client = NexonClient::new().unwrap();
        let body = client
            .request(ENDPOINT, key.trim())
            .await
            .unwrap_or_else(|error| panic!("Live verification failed: {}", error.code));
        let list = parse_character_list(&body)
            .unwrap_or_else(|_| panic!("Live schema did not match required identity fields"));
        let value: serde_json::Value =
            serde_json::from_slice(&body).unwrap_or_else(|_| panic!("Live JSON parse failed"));
        let account_count = list.account_list.len();
        let character_count: usize = list
            .account_list
            .iter()
            .map(|a| a.character_list.len())
            .sum();
        let mut extras = std::collections::BTreeSet::new();
        let mut missing = std::collections::BTreeSet::new();
        fn inspect(
            object: &serde_json::Value,
            expected: &[&str],
            prefix: &str,
            extras: &mut std::collections::BTreeSet<String>,
            missing: &mut std::collections::BTreeSet<String>,
        ) {
            if let Some(object) = object.as_object() {
                for name in object.keys() {
                    if !expected.contains(&name.as_str()) {
                        extras.insert(format!("{prefix}.{name}"));
                    }
                }
                for name in expected {
                    if !object.contains_key(*name) || object[*name].is_null() {
                        missing.insert(format!("{prefix}.{name}"));
                    }
                }
            }
        }
        inspect(&value, &["account_list"], "root", &mut extras, &mut missing);
        for account in value["account_list"].as_array().unwrap() {
            inspect(
                account,
                &["account_id", "character_list"],
                "account",
                &mut extras,
                &mut missing,
            );
            for character in account["character_list"].as_array().unwrap() {
                inspect(
                    character,
                    &[
                        "ocid",
                        "character_name",
                        "world_name",
                        "character_class",
                        "character_level",
                    ],
                    "character",
                    &mut extras,
                    &mut missing,
                );
            }
        }
        // Aggregate counts and schema names only. No identifiers or character values.
        println!("HTTP 200; accounts={account_count}; characters={character_count}; extra_fields={extras:?}; missing_or_null_fields={missing:?}");
    });
}
