use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::Mutex;

use crate::nexon::{ApiError, CharacterList, NexonClient};
pub mod dashboard;

#[derive(Default)]
pub struct CharacterService(pub Mutex<CharacterSession>);

#[derive(Default)]
pub struct CharacterSession {
    client: Option<NexonClient>,
    next_request: Option<Instant>,
    pub(crate) snapshot: Option<CharacterList>,
    pub(crate) version: u64,
    pub(crate) dashboard: dashboard::DashboardCache,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_blocks_before_creating_client_and_reset_clears_it() {
        tauri::async_runtime::block_on(async {
            let service = CharacterService::default();
            let mut session = service.0.lock().await;
            assert!(service.0.try_lock().is_err());
            session.next_request = Some(Instant::now() + Duration::from_secs(120));
            let error = session.load("synthetic-unused-key").await.err().unwrap();
            assert_eq!(error.code, "rate_limited");
            assert!(error.retry_after_seconds.unwrap() >= 119);
            assert!(session.client.is_none()); // No HTTP request was possible.
            let error = session
                .load_scheduler("synthetic-unused-key", "synthetic", "2026-10-28")
                .await
                .err()
                .unwrap();
            assert_eq!(error.code, "rate_limited");
            assert!(session.client.is_none());
            session.reset();
            assert!(session.next_request.is_none());
        });
    }
}

#[derive(Serialize)]
pub struct CharacterSnapshot {
    pub accounts: Vec<crate::nexon::Account>,
    pub version: u64,
}

impl CharacterSession {
    pub fn reset(&mut self) {
        self.next_request = None;
        self.snapshot = None;
        self.version = self.version.saturating_add(1);
        self.dashboard = dashboard::DashboardCache::default();
    }

    pub async fn load(&mut self, key: &str) -> Result<CharacterSnapshot, ApiError> {
        if let Some(next) = self.next_request {
            if let Some(wait) = next.checked_duration_since(Instant::now()) {
                return Err(ApiError {
                    code: "rate_limited",
                    retry_after_seconds: Some(wait.as_secs().saturating_add(1)),
                });
            }
        }
        // Local click throttling, not a claim about Nexon's quota.
        self.next_request = Some(Instant::now() + Duration::from_secs(2));
        if self.client.is_none() {
            self.client = Some(NexonClient::new()?);
        }
        let result = self.client.as_ref().unwrap().character_list(key).await;
        if let Err(error) = &result {
            if error.code == "rate_limited" {
                // checked_add also handles maliciously large Retry-After values safely.
                let wait = Duration::from_secs(error.retry_after_seconds.unwrap_or(60));
                self.next_request = Some(
                    Instant::now()
                        .checked_add(wait)
                        .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400)),
                );
            }
        }
        match result {
            Ok(list) => {
                self.version = self.version.saturating_add(1);
                self.snapshot = Some(list.clone());
                Ok(CharacterSnapshot {
                    accounts: list.account_list,
                    version: self.version,
                })
            }
            Err(error) => {
                if error.code == "authentication" {
                    self.reset();
                }
                Err(error)
            }
        }
    }
}

impl CharacterSession {
    pub async fn load_scheduler(
        &mut self,
        key: &str,
        ocid: &str,
        date: &str,
    ) -> Result<crate::nexon::scheduler::SchedulerState, ApiError> {
        if let Some(wait) = self
            .next_request
            .and_then(|n| n.checked_duration_since(Instant::now()))
        {
            return Err(ApiError {
                code: "rate_limited",
                retry_after_seconds: Some(wait.as_secs().saturating_add(1)),
            });
        }
        self.next_request = Some(Instant::now() + Duration::from_secs(2));
        if self.client.is_none() {
            self.client = Some(NexonClient::new()?);
        }
        let result = self
            .client
            .as_ref()
            .unwrap()
            .scheduler(key, ocid, date)
            .await;
        if let Err(error) = &result {
            if error.code == "rate_limited" {
                self.next_request = Some(
                    Instant::now()
                        .checked_add(Duration::from_secs(error.retry_after_seconds.unwrap_or(60)))
                        .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400)),
                );
            }
        }
        result
    }
}
