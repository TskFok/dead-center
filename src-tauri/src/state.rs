use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;

use crate::config::AppSettings;
use crate::monitor::MonitorInfo;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionType {
    Windows,
    Macos,
    X11,
    Wayland,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub visible: bool,
    pub resolved_monitor_id: String,
    pub using_fallback_monitor: bool,
    pub session_type: SessionType,
    pub warning: Option<String>,
    pub error: Option<String>,
}

impl RuntimeStatus {
    pub fn new(visible: bool) -> Self {
        let session_type = current_session_type();
        let warning = matches!(session_type, SessionType::Wayland)
            .then(|| "Wayland 可能限制全局置顶和全局快捷键；建议使用 X11 会话。".into());
        Self {
            visible,
            resolved_monitor_id: String::new(),
            using_fallback_monitor: false,
            session_type,
            warning,
            error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub settings: AppSettings,
    pub status: RuntimeStatus,
    pub monitors: Vec<MonitorInfo>,
}

pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub status: Mutex<RuntimeStatus>,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        let visible = settings.show_on_launch;
        Self {
            settings: Mutex::new(settings),
            status: Mutex::new(RuntimeStatus::new(visible)),
        }
    }
}

pub fn load_settings<R: Runtime>(app: &AppHandle<R>) -> Result<AppSettings, String> {
    preflight_settings_file(app)?;
    let store = app
        .store("settings.json")
        .map_err(|error| error.to_string())?;

    if let Some(value) = store.get("settings") {
        if let Ok(mut settings) = serde_json::from_value::<AppSettings>(value) {
            settings.visual.normalize();
            return Ok(settings);
        }
    }

    let settings = AppSettings::default();
    store.set(
        "settings",
        serde_json::to_value(&settings).map_err(|error| error.to_string())?,
    );
    store.save().map_err(|error| error.to_string())?;
    Ok(settings)
}

pub fn persist_settings<R: Runtime>(
    app: &AppHandle<R>,
    settings: &AppSettings,
) -> Result<(), String> {
    let store = app
        .store("settings.json")
        .map_err(|error| error.to_string())?;
    store.set(
        "settings",
        serde_json::to_value(settings).map_err(|error| error.to_string())?,
    );
    store.save().map_err(|error| error.to_string())
}

fn preflight_settings_file<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let path = settings_path(app)?;
    let Ok(bytes) = fs::read(&path) else {
        return Ok(());
    };

    if is_valid_settings_store(&bytes) {
        return Ok(());
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let backup = path.with_file_name(format!("settings.corrupt-{timestamp}.json"));
    fs::rename(path, backup).map_err(|error| error.to_string())
}

fn is_valid_settings_store(bytes: &[u8]) -> bool {
    serde_json::from_slice::<HashMap<String, Value>>(bytes)
        .ok()
        .map(|values| {
            values
                .get("settings")
                .map(|value| {
                    serde_json::from_value::<AppSettings>(value.clone())
                        .map(|settings| settings.version == 1)
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .unwrap_or(false)
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("settings.json"))
        .map_err(|error| error.to_string())
}

fn current_session_type() -> SessionType {
    #[cfg(target_os = "windows")]
    return SessionType::Windows;
    #[cfg(target_os = "macos")]
    return SessionType::Macos;
    #[cfg(target_os = "linux")]
    {
        if std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
        {
            SessionType::Wayland
        } else {
            SessionType::X11
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_store_contains_version_one_settings() {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "settings": AppSettings::default()
        }))
        .unwrap();

        assert!(is_valid_settings_store(&bytes));
    }

    #[test]
    fn malformed_json_is_not_a_valid_settings_store() {
        assert!(!is_valid_settings_store(b"{not-json"));
    }

    #[test]
    fn unsupported_settings_version_is_not_valid() {
        let settings = AppSettings {
            version: 2,
            ..AppSettings::default()
        };
        let bytes = serde_json::to_vec(&serde_json::json!({ "settings": settings })).unwrap();

        assert!(!is_valid_settings_store(&bytes));
    }
}
