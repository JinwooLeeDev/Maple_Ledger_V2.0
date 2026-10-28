mod commands;
mod domain;
mod nexon;
mod security;
mod services;
mod storage;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .manage(commands::CredentialState::default())
        .manage(services::CharacterService::default())
        .setup(|app| {
            let path = app
                .path()
                .app_local_data_dir()
                .map(|directory| directory.join("ledger.sqlite3"))
                .map_err(|_| nexon::ApiError::new("settings_unavailable"));
            // Isolated native UI verification, available only in development builds.
            // Never redirect a release build or accept a path outside the OS temp dir.
            #[cfg(debug_assertions)]
            let path = match std::env::var_os("MAPLE_LEDGER_CHECK_DIR") {
                Some(directory) => std::fs::canonicalize(directory)
                    .ok()
                    .filter(|p| {
                        std::fs::canonicalize(std::env::temp_dir())
                            .is_ok_and(|temp| p.starts_with(temp) && p != &std::env::temp_dir())
                    })
                    .map(|p| p.join("ledger.sqlite3"))
                    .ok_or_else(|| nexon::ApiError::new("settings_unavailable")),
                None => path,
            };
            app.manage(commands::SettingsState(path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::api_key_status,
            commands::save_api_key,
            commands::delete_api_key,
            commands::load_characters,
            commands::get_main_character,
            commands::set_main_character,
            commands::save_managed_characters,
            commands::get_dashboard,
            commands::refresh_dashboard_card,
            commands::get_boss_view,
            commands::save_boss_plan,
            commands::save_boss_clear,
            commands::check_boss_api,
        ])
        .run(tauri::generate_context!())
        .expect("Maple Ledger could not start");
}
