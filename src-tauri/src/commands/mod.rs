use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::State;
use zeroize::Zeroizing;
mod dashboard;
pub use dashboard::*;
mod bosses;
pub use bosses::*;

use crate::nexon::ApiError;
use crate::security::{ApiKeyStore, CredentialError};
use crate::services::{CharacterService, CharacterSnapshot};
use crate::{
    domain::{attach_managed, candidate, main_view, MainIdentity, MainView},
    storage::MainRepository,
};

pub struct SettingsState(pub Result<std::path::PathBuf, ApiError>);

async fn settings_access<T: Send + 'static>(
    state: &SettingsState,
    operation: impl FnOnce(MainRepository) -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    let path = state.0.clone()?;
    tauri::async_runtime::spawn_blocking(move || operation(MainRepository::new(path)))
        .await
        .map_err(|_| ApiError::new("settings_unavailable"))?
}

#[derive(Default)]
pub struct CredentialState(Arc<Mutex<ApiKeyStore>>);

#[derive(Serialize)]
pub struct ApiKeyStatus {
    registered: bool,
}

async fn access<T: Send + 'static>(
    state: &CredentialState,
    operation: impl FnOnce(&ApiKeyStore) -> Result<T, CredentialError> + Send + 'static,
) -> Result<T, CredentialError> {
    let store = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        // Serialize access to this credential; never hold the UI thread during OS I/O.
        let store = store.lock().map_err(|_| CredentialError::unavailable())?;
        operation(&store)
    })
    .await
    .map_err(|_| CredentialError::unavailable())?
}

#[tauri::command]
pub async fn api_key_status(
    state: State<'_, CredentialState>,
) -> Result<ApiKeyStatus, CredentialError> {
    access(&state, |store| {
        store
            .is_registered()
            .map(|registered| ApiKeyStatus { registered })
    })
    .await
}

#[tauri::command]
pub async fn save_api_key(
    api_key: String,
    state: State<'_, CredentialState>,
    characters: State<'_, CharacterService>,
) -> Result<ApiKeyStatus, CredentialError> {
    let mut session = characters.0.lock().await;
    let key = Zeroizing::new(api_key);
    let result = access(&state, move |store| {
        store.save(&key)?;
        Ok(ApiKeyStatus { registered: true })
    })
    .await;
    if result.is_ok() {
        session.reset();
    }
    result
}

#[tauri::command]
pub async fn delete_api_key(
    state: State<'_, CredentialState>,
    characters: State<'_, CharacterService>,
) -> Result<ApiKeyStatus, CredentialError> {
    let mut session = characters.0.lock().await;
    let result = access(&state, |store| {
        store.delete()?;
        Ok(ApiKeyStatus { registered: false })
    })
    .await;
    if result.is_ok() {
        session.reset();
    }
    result
}

#[tauri::command]
pub async fn load_characters(
    state: State<'_, CredentialState>,
    characters: State<'_, CharacterService>,
) -> Result<CharacterSnapshot, ApiError> {
    let mut session = characters
        .0
        .try_lock()
        .map_err(|_| ApiError::new("request_in_progress"))?;
    let key = match access(&state, |store| store.read()).await {
        Ok(Some(key)) => key,
        result => {
            session.reset();
            return Err(ApiError::new(if result.is_ok() {
                "key_missing"
            } else {
                "credential_store_unavailable"
            }));
        }
    };
    session.load(&key).await
}

#[tauri::command]
pub async fn get_main_character(
    settings: State<'_, SettingsState>,
    characters: State<'_, CharacterService>,
) -> Result<MainView, ApiError> {
    let session = characters.0.lock().await;
    let configuration = settings_access(&settings, |repo| repo.load_configuration()).await?;
    Ok(attach_managed(
        main_view(
            configuration.main,
            session.snapshot.as_ref(),
            session.snapshot.as_ref().map(|_| session.version),
        ),
        &configuration.managed,
        session.snapshot.as_ref(),
    ))
}

#[tauri::command]
pub async fn set_main_character(
    ocid: String,
    snapshot_version: u64,
    expected_main: Option<MainIdentity>,
    expected_selection_revision: i64,
    confirmed: bool,
    settings: State<'_, SettingsState>,
    characters: State<'_, CharacterService>,
) -> Result<MainView, ApiError> {
    let session = characters.0.lock().await;
    if session.version != snapshot_version {
        return Err(ApiError::new("snapshot_changed"));
    }
    let list = session
        .snapshot
        .as_ref()
        .ok_or_else(|| ApiError::new("no_character_data"))?;
    let next = candidate(list, &ocid)?;
    let owned_list = list.clone();
    let saved = settings_access(&settings, move |repo| {
        repo.save(
            &next,
            expected_main.as_ref(),
            confirmed,
            &owned_list,
            expected_selection_revision,
        )
    })
    .await?;
    Ok(attach_managed(
        main_view(saved.main, Some(list), Some(session.version)),
        &saved.managed,
        Some(list),
    ))
}

#[tauri::command]
pub async fn save_managed_characters(
    ocids: Vec<String>,
    snapshot_version: Option<u64>,
    expected_main: MainIdentity,
    expected_selection_revision: i64,
    settings: State<'_, SettingsState>,
    characters: State<'_, CharacterService>,
) -> Result<MainView, ApiError> {
    let session = characters.0.lock().await;
    if snapshot_version.is_some_and(|version| version != session.version) {
        return Err(ApiError::new("snapshot_changed"));
    }
    // Without a verified snapshot, only retaining/removing existing entries is allowed.
    let owned_list = snapshot_version.and(session.snapshot.clone());
    let saved = settings_access(&settings, move |repo| {
        repo.save_managed(
            &ocids,
            &expected_main,
            expected_selection_revision,
            owned_list.as_ref(),
        )
    })
    .await?;
    Ok(attach_managed(
        main_view(
            saved.main,
            session.snapshot.as_ref(),
            session.snapshot.as_ref().map(|_| session.version),
        ),
        &saved.managed,
        session.snapshot.as_ref(),
    ))
}
