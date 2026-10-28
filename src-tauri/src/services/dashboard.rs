use std::{
    collections::HashMap,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use super::CharacterSession;
use crate::{
    domain::{managed::ManagedSelection, MainCharacter},
    nexon::{basic::BasicInfo, ApiError, NexonClient},
};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct DashboardCard {
    pub character: MainCharacter,
    pub is_main: bool,
    pub image_src: Option<String>,
    pub updated_at: Option<u64>,
    pub error: Option<ApiError>,
    pub image_error: bool,
}

#[derive(Serialize)]
pub struct DashboardSnapshot {
    pub cards: Vec<DashboardCard>,
    pub selection_revision: i64,
}

struct CachedCard {
    card: DashboardCard,
    image_url: Option<String>,
    attempted_at: Instant,
}

#[derive(Default)]
pub struct DashboardCache {
    entries: HashMap<String, CachedCard>,
    blocked_until: Option<Instant>,
}

impl DashboardCache {
    pub fn snapshot(&mut self, selection: &ManagedSelection) -> DashboardSnapshot {
        self.entries.retain(|ocid, cached| {
            selection
                .characters
                .iter()
                .any(|c| &c.identity.ocid == ocid && c.identity == cached.card.character.identity)
        });
        DashboardSnapshot {
            cards: selection
                .characters
                .iter()
                .enumerate()
                .map(|(index, saved)| self.card(saved, index == 0))
                .collect(),
            selection_revision: selection.revision,
        }
    }

    pub fn card(&self, saved: &MainCharacter, is_main: bool) -> DashboardCard {
        match self
            .entries
            .get(&saved.identity.ocid)
            .filter(|entry| entry.card.character.identity == saved.identity)
        {
            Some(entry) => {
                let mut card = entry.card.clone();
                card.is_main = is_main;
                card
            }
            None => DashboardCard {
                character: saved.clone(),
                is_main,
                image_src: None,
                updated_at: None,
                error: None,
                image_error: false,
            },
        }
    }

    pub fn needs_refresh(&self, saved: &MainCharacter, force: bool) -> bool {
        let Some(entry) = self
            .entries
            .get(&saved.identity.ocid)
            .filter(|entry| entry.card.character.identity == saved.identity)
        else {
            return true;
        };
        let duration = if force {
            2
        } else if entry.card.error.is_some() || entry.card.image_error {
            60
        } else {
            15 * 60
        };
        entry.attempted_at.elapsed() >= Duration::from_secs(duration)
    }

    pub fn failure(
        &mut self,
        saved: &MainCharacter,
        is_main: bool,
        error: ApiError,
    ) -> DashboardCard {
        let mut card = self.card(saved, is_main);
        if error.code == "rate_limited" {
            self.blocked_until = Instant::now()
                .checked_add(Duration::from_secs(error.retry_after_seconds.unwrap_or(60)));
        }
        card.error = Some(error);
        let image_url = self
            .entries
            .get(&saved.identity.ocid)
            .and_then(|entry| entry.image_url.clone());
        self.entries.insert(
            saved.identity.ocid.clone(),
            CachedCard {
                card: card.clone(),
                image_url,
                attempted_at: Instant::now(),
            },
        );
        card
    }
}

fn apply_basic(card: &mut DashboardCard, info: &BasicInfo) -> Result<(), ApiError> {
    if info.world_name != card.character.identity.world_name {
        return Err(ApiError::new("identity_changed"));
    }
    if let Some(name) = info
        .character_name
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        card.character.character_name = Some(name.clone());
    }
    if let Some(class) = info
        .character_class
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        card.character.character_class = Some(class.clone());
    }
    if info.character_level.is_some() {
        card.character.character_level = info.character_level;
    }
    card.updated_at = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    );
    card.error = None;
    Ok(())
}

impl CharacterSession {
    pub async fn refresh_card(
        &mut self,
        saved: &MainCharacter,
        is_main: bool,
        key: &str,
        force: bool,
    ) -> DashboardCard {
        if !self.dashboard.needs_refresh(saved, force) {
            return self.dashboard.card(saved, is_main);
        }
        if let Some(wait) = self
            .dashboard
            .blocked_until
            .and_then(|next| next.checked_duration_since(Instant::now()))
        {
            return self.dashboard.failure(
                saved,
                is_main,
                ApiError {
                    code: "rate_limited",
                    retry_after_seconds: Some(wait.as_secs().saturating_add(1)),
                },
            );
        }
        if self.client.is_none() {
            match NexonClient::new() {
                Ok(client) => self.client = Some(client),
                Err(error) => return self.dashboard.failure(saved, is_main, error),
            }
        }
        let info = match self
            .client
            .as_ref()
            .unwrap()
            .character_basic(key, &saved.identity.ocid)
            .await
        {
            Ok(info) => info,
            Err(error) => return self.dashboard.failure(saved, is_main, error),
        };
        let mut card = self.dashboard.card(saved, is_main);
        if let Err(error) = apply_basic(&mut card, &info) {
            return self.dashboard.failure(saved, is_main, error);
        }
        let previous = self.dashboard.entries.get(&saved.identity.ocid);
        let same_image = previous.is_some_and(|entry| {
            entry.image_url == info.character_image && entry.card.image_src.is_some()
        });
        let mut image_url = previous.and_then(|entry| entry.image_url.clone());
        if !same_image {
            match info.character_image.as_deref() {
                Some(url) => match self.client.as_ref().unwrap().character_image(url).await {
                    Ok(image) => {
                        card.image_src = Some(image);
                        card.image_error = false;
                        image_url = info.character_image.clone();
                    }
                    Err(error) => {
                        card.image_error = true;
                        if error.code == "rate_limited" {
                            self.dashboard.blocked_until = Instant::now().checked_add(
                                Duration::from_secs(error.retry_after_seconds.unwrap_or(60)),
                            );
                        }
                    }
                },
                None => card.image_error = true,
            }
        } else {
            card.image_error = false;
        }
        self.dashboard.entries.insert(
            saved.identity.ocid.clone(),
            CachedCard {
                card: card.clone(),
                image_url,
                attempted_at: Instant::now(),
            },
        );
        card
    }
}

#[cfg(test)]
mod tests;
