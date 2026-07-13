use std::path::PathBuf;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::{
    monitor::{collect_monitors, full_screen_overlay_geometry, monitor_infos, resolve_monitor},
    state::{AppSnapshot, AppState},
};

pub const OVERLAY_LABEL: &str = "crosshair";

pub fn create_or_refresh_overlay<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let should_show = app
        .state::<AppState>()
        .status
        .lock()
        .map_err(|error| error.to_string())?
        .visible;
    let created = app.get_webview_window(OVERLAY_LABEL).is_none();

    if created {
        let mut builder = WebviewWindowBuilder::new(
            app,
            OVERLAY_LABEL,
            WebviewUrl::App(PathBuf::from("index.html?view=overlay")),
        )
        .title("Dead Center Crosshair")
        .inner_size(1.0, 1.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .focusable(false)
        .skip_taskbar(true)
        .visible(false);

        #[cfg(target_os = "macos")]
        {
            builder = builder.visible_on_all_workspaces(true);
        }

        let window = builder.build().map_err(|error| error.to_string())?;
        window
            .set_ignore_cursor_events(true)
            .map_err(|error| error.to_string())?;
    }

    refresh_overlay_geometry(app)?;
    if created && should_show {
        if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
            window.show().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub fn refresh_overlay_geometry<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let monitors = collect_monitors(app)?;
    if monitors.is_empty() {
        return Err("没有检测到可用显示器".into());
    }
    let target = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .target_monitor_id
        .clone();
    let resolved = resolve_monitor(&monitors, target.as_deref());
    let geometry = full_screen_overlay_geometry(resolved.monitor)
        .ok_or_else(|| "目标显示器缩放比例无效".to_string())?;

    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        window
            .set_position(PhysicalPosition::new(
                geometry.position.x,
                geometry.position.y,
            ))
            .map_err(|error| error.to_string())?;
        window
            .set_size(PhysicalSize::new(
                geometry.physical_width,
                geometry.physical_height,
            ))
            .map_err(|error| error.to_string())?;
        window
            .set_always_on_top(true)
            .map_err(|error| error.to_string())?;
        window
            .set_focusable(false)
            .map_err(|error| error.to_string())?;
    }

    let app_state = app.state::<AppState>();
    let mut status = app_state.status.lock().map_err(|error| error.to_string())?;
    status.resolved_monitor_id = resolved.monitor.id.clone();
    status.using_fallback_monitor = resolved.using_fallback;
    status.error = None;
    let status_payload = status.clone();
    drop(status);
    let _ = app.emit_to(OVERLAY_LABEL, "crosshair://runtime-changed", status_payload);
    Ok(())
}

pub fn emit_visual<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let visual = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .visual
        .clone();
    app.emit_to(OVERLAY_LABEL, "crosshair://visual-changed", visual)
        .map_err(|error| error.to_string())
}

pub fn set_visibility<R: Runtime>(app: &AppHandle<R>, visible: bool) -> Result<(), String> {
    if visible {
        create_or_refresh_overlay(app)?;
    }
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        if visible {
            window.show().map_err(|error| error.to_string())?;
            window
                .set_always_on_top(true)
                .map_err(|error| error.to_string())?;
        } else {
            window.hide().map_err(|error| error.to_string())?;
        }
    }
    let app_state = app.state::<AppState>();
    let mut status = app_state.status.lock().map_err(|error| error.to_string())?;
    status.visible = visible;
    let status_payload = status.clone();
    drop(status);
    let _ = app.emit("crosshair://runtime-changed", status_payload);
    Ok(())
}

pub fn snapshot<R: Runtime>(app: &AppHandle<R>) -> Result<AppSnapshot, String> {
    let monitors = collect_monitors(app)?;
    let settings = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .clone();
    let status = app
        .state::<AppState>()
        .status
        .lock()
        .map_err(|error| error.to_string())?
        .clone();
    Ok(AppSnapshot {
        settings,
        status,
        monitors: monitor_infos(&monitors),
    })
}

pub fn record_overlay_error<R: Runtime>(app: &AppHandle<R>, error: String) {
    if let Ok(mut status) = app.state::<AppState>().status.lock() {
        status.error = Some(error);
    }
}
