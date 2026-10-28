use super::{settings_access, SettingsState};
use crate::{
    domain::bosses::{now, ClearInput, ClearSnapshot},
    nexon::ApiError,
    storage::bosses::{BossView, PlanInput},
};
use tauri::State;

#[tauri::command]
pub async fn get_boss_view(
    ocid: Option<String>,
    week: Option<i64>,
    settings: State<'_, SettingsState>,
) -> Result<BossView, ApiError> {
    settings_access(&settings, move |repo| {
        repo.boss_view(ocid.as_deref(), week, now())
    })
    .await
}
#[tauri::command]
pub async fn save_boss_plan(
    input: PlanInput,
    settings: State<'_, SettingsState>,
) -> Result<(), ApiError> {
    settings_access(&settings, move |repo| repo.save_boss_plan(input)).await
}
#[tauri::command]
pub async fn save_boss_clear(
    input: ClearInput,
    settings: State<'_, SettingsState>,
) -> Result<ClearSnapshot, ApiError> {
    settings_access(&settings, move |repo| repo.save_boss_clear(input, now())).await
}

#[tauri::command]
pub async fn check_boss_api(
    ocid: String,
    week: i64,
    selection_revision: i64,
    settings: State<'_, SettingsState>,
    credentials: State<'_, super::CredentialState>,
    characters: State<'_, crate::services::CharacterService>,
) -> Result<crate::storage::bosses::Observation, ApiError> {
    let mut session = characters
        .0
        .try_lock()
        .map_err(|_| ApiError::new("request_in_progress"))?;
    #[cfg(debug_assertions)]
    if std::env::var_os("MAPLE_LEDGER_OFFLINE_CHECK").is_some() {
        return Err(ApiError::new("offline_check"));
    }
    let id = ocid.clone();
    settings_access(&settings, move |repo| {
        let config = repo.load_configuration()?;
        if config.managed.revision != selection_revision {
            return Err(ApiError::new("selection_changed"));
        }
        if !config
            .managed
            .characters
            .iter()
            .any(|c| c.identity.ocid == id)
        {
            return Err(ApiError::new("boss_not_managed"));
        }
        if week != crate::domain::bosses::week_start(now()) {
            return Err(ApiError::new("boss_api_stale"));
        }
        Ok(())
    })
    .await?;
    let key = super::access(&credentials, |store| store.read())
        .await
        .map_err(|_| ApiError::new("credential_store_unavailable"))?
        .ok_or_else(|| ApiError::new("key_missing"))?;
    let date = crate::domain::bosses::date(crate::domain::bosses::kst_day(now()));
    let state = session.load_scheduler(&key, &ocid, &date).await?;
    settings_access(&settings, move |repo| {
        repo.store_boss_observation(&ocid, selection_revision, &date, state, now())
    })
    .await
}
