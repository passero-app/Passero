mod config;
mod error;
#[cfg(not(target_os = "ios"))]
mod git;
#[cfg(not(target_os = "ios"))]
mod gpg;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(not(target_os = "ios"))]
mod pass;
#[cfg(not(target_os = "ios"))]
mod path;
#[cfg(not(target_os = "ios"))]
mod totp;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_theme(Some(tauri::Theme::Dark));
                }
            }

            #[cfg(target_os = "ios")]
            {
                use tauri::Manager;
                let data_dir = app.path().app_data_dir()?;
                let store_dir = data_dir.join("store");
                let _ = std::fs::create_dir_all(&store_dir);
                let state = app.state::<ios::IosState>();
                *state.store_dir.lock().unwrap() = store_dir;
            }

            Ok(())
        });

    #[cfg(not(target_os = "ios"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        pass::commands::list_passwords,
        pass::commands::show_password,
        pass::commands::insert_password,
        pass::commands::edit_password,
        pass::commands::delete_password,
        pass::commands::generate_password,
        pass::commands::copy_password,
        pass::commands::list_recipients,
        pass::commands::add_recipient,
        pass::commands::remove_recipient,
        pass::commands::init_password_store,
        gpg::commands::list_gpg_keys,
        gpg::commands::list_gpg_secret_keys,
        gpg::commands::get_store_gpg_id,
        gpg::commands::generate_gpg_key,
        gpg::commands::import_gpg_key,
        gpg::commands::import_gpg_key_from_keyserver,
        gpg::commands::export_gpg_key,
        gpg::commands::publish_gpg_key,
        gpg::commands::set_gpg_key_trust,
        gpg::commands::delete_gpg_key,
        gpg::commands::resolve_gpg_keys,
        gpg::commands::search_gpg_keyserver,
        git::commands::git_pull,
        git::commands::git_push,
        git::commands::git_log,
        git::commands::git_clone,
        config::commands::get_config,
        config::commands::set_config,
        config::commands::get_password_store_path,
        totp::commands::get_totp,
        totp::commands::get_totp_info,
        totp::commands::insert_totp,
        totp::commands::decode_qr_image,
        totp::commands::import_totp_from_qr,
        config::commands::list_vaults,
        config::commands::add_vault,
        config::commands::remove_vault,
        config::commands::set_active_vault,
    ]);

    #[cfg(target_os = "ios")]
    let builder = builder
        .plugin(tauri_plugin_keystore::init())
        .plugin(tauri_plugin_biometric::init())
        .manage(ios::IosState::default())
        .invoke_handler(tauri::generate_handler![
            ios::list_passwords,
            ios::show_password,
            ios::insert_password,
            ios::edit_password,
            ios::delete_password,
            ios::generate_password,
            ios::copy_password,
            ios::list_recipients,
            ios::add_recipient,
            ios::remove_recipient,
            ios::init_password_store,
            ios::generate_in_app_key,
            ios::device_key_status,
            ios::reset_device,
            ios::load_key,
            ios::clone_store,
            ios::get_sync_settings,
            ios::set_sync_settings,
            ios::store_initialized,
            ios::init_store,
            ios::list_gpg_keys,
            ios::list_gpg_secret_keys,
            ios::get_store_gpg_id,
            ios::generate_gpg_key,
            ios::import_gpg_key,
            ios::import_gpg_key_from_keyserver,
            ios::export_gpg_key,
            ios::publish_gpg_key,
            ios::set_gpg_key_trust,
            ios::delete_gpg_key,
            ios::resolve_gpg_keys,
            ios::search_gpg_keyserver,
            ios::git_pull,
            ios::git_push,
            ios::git_log,
            ios::git_clone,
            ios::get_totp,
            ios::get_totp_info,
            ios::insert_totp,
            ios::decode_qr_image,
            ios::import_totp_from_qr,
            config::commands::get_config,
            config::commands::set_config,
            config::commands::get_password_store_path,
            config::commands::list_vaults,
            config::commands::add_vault,
            config::commands::remove_vault,
            config::commands::set_active_vault,
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
