mod config;
mod error;
mod git;
mod gpg;
mod pass;
mod path;
mod totp;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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

            // Force dark title bar
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_theme(Some(tauri::Theme::Dark));
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
