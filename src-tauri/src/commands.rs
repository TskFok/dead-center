use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::{
    config::{AppSettings, VisualSettings},
    overlay,
    state::{persist_settings, AppSnapshot, AppState},
};

fn settings_copy(state: &State<'_, AppState>) -> Result<AppSettings, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|error| error.to_string())
}

fn save_state<R: Runtime>(app: &AppHandle<R>, state: &State<'_, AppState>) -> Result<(), String> {
    persist_settings(app, &settings_copy(state)?)
}

#[tauri::command]
pub fn get_snapshot<R: Runtime>(app: AppHandle<R>) -> Result<AppSnapshot, String> {
    overlay::snapshot(&app)
}

#[tauri::command]
pub fn update_visual<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    mut visual: VisualSettings,
) -> Result<AppSnapshot, String> {
    visual.normalize();
    let previous = {
        let mut settings = state.settings.lock().map_err(|error| error.to_string())?;
        std::mem::replace(&mut settings.visual, visual)
    };
    if let Err(error) = save_state(&app, &state) {
        state
            .settings
            .lock()
            .map_err(|error| error.to_string())?
            .visual = previous;
        return Err(error);
    }
    overlay::emit_visual(&app)?;
    overlay::snapshot(&app)
}

#[tauri::command]
pub fn select_monitor<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    monitor_id: Option<String>,
) -> Result<AppSnapshot, String> {
    let previous = {
        let mut settings = state.settings.lock().map_err(|error| error.to_string())?;
        std::mem::replace(&mut settings.target_monitor_id, monitor_id)
    };
    if let Err(error) = save_state(&app, &state) {
        state
            .settings
            .lock()
            .map_err(|error| error.to_string())?
            .target_monitor_id = previous;
        return Err(error);
    }
    overlay::refresh_overlay_position(&app)?;
    overlay::snapshot(&app)
}

#[tauri::command]
pub fn set_toggle_shortcut<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    shortcut: String,
) -> Result<AppSnapshot, String> {
    let old = settings_copy(&state)?.toggle_shortcut;
    if old == shortcut {
        return overlay::snapshot(&app);
    }

    app.global_shortcut()
        .register(shortcut.as_str())
        .map_err(|error| format!("快捷键不可用：{error}"))?;
    if let Err(error) = app.global_shortcut().unregister(old.as_str()) {
        let _ = app.global_shortcut().unregister(shortcut.as_str());
        return Err(format!("无法替换旧快捷键：{error}"));
    }

    state
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .toggle_shortcut = shortcut.clone();
    if let Err(error) = save_state(&app, &state) {
        let _ = app.global_shortcut().unregister(shortcut.as_str());
        let _ = app.global_shortcut().register(old.as_str());
        state
            .settings
            .lock()
            .map_err(|error| error.to_string())?
            .toggle_shortcut = old;
        return Err(error);
    }
    overlay::snapshot(&app)
}

#[tauri::command]
pub fn set_launch_at_login<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<AppSnapshot, String> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|error| format!("无法更新开机启动：{error}"))?;

    state
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .launch_at_login = enabled;
    save_state(&app, &state)?;
    overlay::snapshot(&app)
}

#[tauri::command]
pub fn set_show_on_launch<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<AppSnapshot, String> {
    state
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .show_on_launch = enabled;
    save_state(&app, &state)?;
    overlay::snapshot(&app)
}

#[tauri::command]
pub fn set_visibility<R: Runtime>(app: AppHandle<R>, visible: bool) -> Result<AppSnapshot, String> {
    overlay::set_visibility(&app, visible)?;
    overlay::snapshot(&app)
}

#[tauri::command]
pub fn retry_overlay<R: Runtime>(app: AppHandle<R>) -> Result<AppSnapshot, String> {
    overlay::create_or_refresh_overlay(&app)?;
    overlay::emit_visual(&app)?;
    overlay::snapshot(&app)
}

pub fn toggle_visibility<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let visible = app
        .state::<AppState>()
        .status
        .lock()
        .map_err(|error| error.to_string())?
        .visible;
    overlay::set_visibility(app, !visible)
}

pub fn show_settings<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "设置窗口不存在".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    let _ = window.unminimize();
    window.set_focus().map_err(|error| error.to_string())
}

pub fn toggle_autostart<R: Runtime>(app: &AppHandle<R>) -> Result<bool, String> {
    let current = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .launch_at_login;
    let enabled = !current;
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|error| error.to_string())?;
    let settings = {
        let app_state = app.state::<AppState>();
        let mut settings = app_state
            .settings
            .lock()
            .map_err(|error| error.to_string())?;
        settings.launch_at_login = enabled;
        settings.clone()
    };
    persist_settings(app, &settings)?;
    Ok(enabled)
}
