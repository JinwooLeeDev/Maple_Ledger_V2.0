fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "api_key_status",
        "save_api_key",
        "delete_api_key",
        "load_characters",
        "get_main_character",
        "set_main_character",
        "save_managed_characters",
        "get_dashboard",
        "refresh_dashboard_card",
        "get_boss_view",
        "save_boss_plan",
        "save_boss_clear",
        "check_boss_api",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("Could not prepare application permissions");
}
