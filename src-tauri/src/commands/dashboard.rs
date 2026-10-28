use super::{access, settings_access, CredentialState, SettingsState};
use crate::{
    nexon::ApiError,
    services::{
        dashboard::{DashboardCard, DashboardSnapshot},
        CharacterService,
    },
};
use tauri::State;

#[tauri::command]
pub async fn get_dashboard(
    settings: State<'_, SettingsState>,
    characters: State<'_, CharacterService>,
) -> Result<DashboardSnapshot, ApiError> {
    let mut session = characters.0.lock().await;
    let config = settings_access(&settings, |repo| repo.load_configuration()).await?;
    Ok(session.dashboard.snapshot(&config.managed))
}

#[tauri::command]
pub async fn refresh_dashboard_card(
    ocid: String,
    expected_selection_revision: i64,
    force: bool,
    settings: State<'_, SettingsState>,
    characters: State<'_, CharacterService>,
    credentials: State<'_, CredentialState>,
) -> Result<DashboardCard, ApiError> {
    // Explicit debug-only offline UI checks must never contact Nexon or read a key.
    #[cfg(debug_assertions)]
    if std::env::var_os("MAPLE_LEDGER_OFFLINE_CHECK").is_some() {
        return Err(ApiError::new("offline_verification"));
    }
    let mut session = characters.0.lock().await;
    let config = settings_access(&settings, |repo| repo.load_configuration()).await?;
    if config.managed.revision != expected_selection_revision {
        return Err(ApiError::new("selection_changed"));
    }
    session.dashboard.snapshot(&config.managed);
    let (index, saved) = config
        .managed
        .characters
        .iter()
        .enumerate()
        .find(|(_, c)| c.identity.ocid == ocid)
        .ok_or_else(|| ApiError::new("candidate_missing"))?;
    if !session.dashboard.needs_refresh(saved, force) {
        return Ok(session.dashboard.card(saved, index == 0));
    }
    let key = match access(&credentials, |store| store.read()).await {
        Ok(Some(key)) => key,
        result => {
            return Ok(session.dashboard.failure(
                saved,
                index == 0,
                ApiError::new(if result.is_ok() {
                    "key_missing"
                } else {
                    "credential_store_unavailable"
                }),
            ))
        }
    };
    Ok(session.refresh_card(saved, index == 0, &key, force).await)
}
