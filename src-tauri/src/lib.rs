mod commands;
mod config;
mod monitor;
mod overlay;
mod state;
mod tray;

use std::{thread, time::Duration};

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = commands::show_settings(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = commands::toggle_visibility(app);
                    }
                })
                .build(),
        )
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("Dead Center")
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.handle()
                .set_activation_policy(tauri::ActivationPolicy::Accessory)?;

            let settings = state::load_settings(app.handle()).map_err(std::io::Error::other)?;
            app.manage(state::AppState::new(settings.clone()));

            app.global_shortcut()
                .register(settings.toggle_shortcut.as_str())?;
            if settings.launch_at_login {
                let _ = app.autolaunch().enable();
            } else {
                let _ = app.autolaunch().disable();
            }

            if let Err(error) = overlay::create_or_refresh_overlay(app.handle()) {
                overlay::record_overlay_error(app.handle(), error);
            } else {
                let _ = overlay::emit_visual(app.handle());
            }
            tray::create_tray(app)?;

            let handle = app.handle().clone();
            thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(2));
                if let Err(error) = overlay::refresh_overlay_position(&handle) {
                    overlay::record_overlay_error(&handle, error);
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "settings" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::update_visual,
            commands::select_monitor,
            commands::set_toggle_shortcut,
            commands::set_launch_at_login,
            commands::set_show_on_launch,
            commands::set_visibility,
            commands::retry_overlay,
        ])
        .run(tauri::generate_context!())
        .expect("Dead Center 启动失败");
}
