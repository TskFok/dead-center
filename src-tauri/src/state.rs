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

use crate::config::{migrate_v1_settings, AppSettings, LegacyAppSettingsV1, SETTINGS_VERSION};
use crate::monitor::{collect_monitors, resolved_logical_short_edge, MonitorInfo};

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
        if let Some((settings, migrated)) = decode_settings(app, value) {
            if migrated {
                store.set(
                    "settings",
                    serde_json::to_value(&settings).map_err(|error| error.to_string())?,
                );
                store.save().map_err(|error| error.to_string())?;
            }
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
                .cloned()
                .map(is_supported_settings_value)
                .unwrap_or(true)
        })
        .unwrap_or(false)
}

fn settings_version(value: &Value) -> Option<u64> {
    value.get("version").and_then(Value::as_u64)
}

fn legacy_short_edge<R: Runtime>(
    app: &AppHandle<R>,
    target_monitor_id: Option<&str>,
) -> Option<f64> {
    let monitors = collect_monitors(app).ok()?;
    resolved_logical_short_edge(&monitors, target_monitor_id)
}

fn decode_settings<R: Runtime>(app: &AppHandle<R>, value: Value) -> Option<(AppSettings, bool)> {
    match settings_version(&value)? {
        1 => {
            let legacy = serde_json::from_value::<LegacyAppSettingsV1>(value).ok()?;
            let short_edge = legacy_short_edge(app, legacy.target_monitor_id.as_deref());
            Some((migrate_v1_settings(legacy, short_edge), true))
        }
        version if version == u64::from(SETTINGS_VERSION) => {
            let mut settings = serde_json::from_value::<AppSettings>(value).ok()?;
            settings.visual.normalize();
            Some((settings, false))
        }
        _ => None,
    }
}

fn is_supported_settings_value(value: Value) -> bool {
    match settings_version(&value) {
        Some(1) => serde_json::from_value::<LegacyAppSettingsV1>(value).is_ok(),
        Some(version) if version == u64::from(SETTINGS_VERSION) => {
            serde_json::from_value::<AppSettings>(value).is_ok()
        }
        _ => false,
    }
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

    fn version_one_value() -> Value {
        serde_json::json!({
            "version": 1,
            "visual": {
                "preset": "classic-cross",
                "primaryColor": "#4DFFB8",
                "accentColor": "#F4FF4D",
                "opacity": 0.8,
                "sizePx": 32.0,
                "strokePx": 3.0,
                "gapPx": 8.0
            },
            "targetMonitorId": null,
            "toggleShortcut": "Alt+Shift+X",
            "launchAtLogin": false,
            "showOnLaunch": true
        })
    }

    #[test]
    fn valid_store_accepts_versions_one_and_two() {
        for settings in [
            version_one_value(),
            serde_json::to_value(AppSettings::default()).unwrap(),
        ] {
            let bytes = serde_json::to_vec(&serde_json::json!({
                "settings": settings
            }))
            .unwrap();
            assert!(is_valid_settings_store(&bytes));
        }
    }

    #[test]
    fn malformed_json_is_not_a_valid_settings_store() {
        assert!(!is_valid_settings_store(b"{not-json"));
    }

    #[test]
    fn unsupported_settings_version_is_not_valid() {
        let mut settings = serde_json::to_value(AppSettings::default()).unwrap();
        settings["version"] = 3.into();
        let bytes = serde_json::to_vec(&serde_json::json!({ "settings": settings })).unwrap();

        assert!(!is_valid_settings_store(&bytes));
    }
}
