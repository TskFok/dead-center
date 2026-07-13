use std::path::PathBuf;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

use crate::{
    monitor::{
        collect_monitors, full_screen_overlay_geometry, monitor_infos, resolve_monitor,
        OverlayGeometry,
    },
    state::{AppSnapshot, AppState},
};

pub const OVERLAY_LABEL: &str = "crosshair";

trait OverlayWindowOps {
    fn set_physical_position(&self, x: i32, y: i32) -> Result<(), String>;
    fn set_physical_size(&self, width: u32, height: u32) -> Result<(), String>;
    fn ignore_cursor_events(&self) -> Result<(), String>;
    fn keep_always_on_top(&self) -> Result<(), String>;
    fn disable_focus(&self) -> Result<(), String>;
    fn show(&self) -> Result<(), String>;
    fn hide(&self) -> Result<(), String>;
}

impl<R: Runtime> OverlayWindowOps for WebviewWindow<R> {
    fn set_physical_position(&self, x: i32, y: i32) -> Result<(), String> {
        self.set_position(PhysicalPosition::new(x, y))
            .map_err(|error| error.to_string())
    }

    fn set_physical_size(&self, width: u32, height: u32) -> Result<(), String> {
        self.set_size(PhysicalSize::new(width, height))
            .map_err(|error| error.to_string())
    }

    fn ignore_cursor_events(&self) -> Result<(), String> {
        self.set_ignore_cursor_events(true)
            .map_err(|error| error.to_string())
    }

    fn keep_always_on_top(&self) -> Result<(), String> {
        self.set_always_on_top(true)
            .map_err(|error| error.to_string())
    }

    fn disable_focus(&self) -> Result<(), String> {
        self.set_focusable(false).map_err(|error| error.to_string())
    }

    fn show(&self) -> Result<(), String> {
        self.show().map_err(|error| error.to_string())
    }

    fn hide(&self) -> Result<(), String> {
        self.hide().map_err(|error| error.to_string())
    }
}

fn apply_overlay_window_state<W: OverlayWindowOps>(
    window: &W,
    geometry: &OverlayGeometry,
    visible: bool,
) -> Result<(), String> {
    window.set_physical_position(geometry.position.x, geometry.position.y)?;
    window.set_physical_size(geometry.physical_width, geometry.physical_height)?;
    window.ignore_cursor_events()?;
    window.keep_always_on_top()?;
    window.disable_focus()?;
    if visible {
        window.show()
    } else {
        window.hide()
    }
}

pub fn create_or_refresh_overlay<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if app.get_webview_window(OVERLAY_LABEL).is_none() {
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

        builder.build().map_err(|error| error.to_string())?;
    }

    refresh_overlay_geometry(app)
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
    let app_state = app.state::<AppState>();
    let should_show = app_state
        .status
        .lock()
        .map_err(|error| error.to_string())?
        .visible;

    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        apply_overlay_window_state(&window, &geometry, should_show)?;
    }

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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::monitor::{OverlayGeometry, PhysicalPoint};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Operation {
        Position(i32, i32),
        Size(u32, u32),
        IgnoreCursor,
        AlwaysOnTop,
        Focusable,
        Show,
        Hide,
    }

    struct FakeWindow {
        operations: RefCell<Vec<Operation>>,
        fail_once: RefCell<Option<Operation>>,
    }

    impl FakeWindow {
        fn new(fail_once: Option<Operation>) -> Self {
            Self {
                operations: RefCell::new(Vec::new()),
                fail_once: RefCell::new(fail_once),
            }
        }

        fn perform(&self, operation: Operation) -> Result<(), String> {
            self.operations.borrow_mut().push(operation);
            let should_fail = self.fail_once.borrow().as_ref() == Some(&operation);
            if should_fail {
                self.fail_once.borrow_mut().take();
                Err("计划内失败".into())
            } else {
                Ok(())
            }
        }

        fn operations(&self) -> Vec<Operation> {
            self.operations.borrow().clone()
        }
    }

    impl OverlayWindowOps for FakeWindow {
        fn set_physical_position(&self, x: i32, y: i32) -> Result<(), String> {
            self.perform(Operation::Position(x, y))
        }

        fn set_physical_size(&self, width: u32, height: u32) -> Result<(), String> {
            self.perform(Operation::Size(width, height))
        }

        fn ignore_cursor_events(&self) -> Result<(), String> {
            self.perform(Operation::IgnoreCursor)
        }

        fn keep_always_on_top(&self) -> Result<(), String> {
            self.perform(Operation::AlwaysOnTop)
        }

        fn disable_focus(&self) -> Result<(), String> {
            self.perform(Operation::Focusable)
        }

        fn show(&self) -> Result<(), String> {
            self.perform(Operation::Show)
        }

        fn hide(&self) -> Result<(), String> {
            self.perform(Operation::Hide)
        }
    }

    fn geometry() -> OverlayGeometry {
        OverlayGeometry {
            position: PhysicalPoint { x: -2560, y: -180 },
            physical_width: 2560,
            physical_height: 1440,
            logical_width: 1280.0,
            logical_height: 720.0,
        }
    }

    fn safe_show_sequence() -> Vec<Operation> {
        vec![
            Operation::Position(-2560, -180),
            Operation::Size(2560, 1440),
            Operation::IgnoreCursor,
            Operation::AlwaysOnTop,
            Operation::Focusable,
            Operation::Show,
        ]
    }

    #[test]
    fn applies_geometry_and_safety_before_syncing_visibility() {
        let visible_window = FakeWindow::new(None);
        apply_overlay_window_state(&visible_window, &geometry(), true).unwrap();
        assert_eq!(visible_window.operations(), safe_show_sequence());

        let hidden_window = FakeWindow::new(None);
        apply_overlay_window_state(&hidden_window, &geometry(), false).unwrap();
        let mut expected = safe_show_sequence();
        *expected.last_mut().unwrap() = Operation::Hide;
        assert_eq!(hidden_window.operations(), expected);
    }

    #[test]
    fn never_shows_when_any_pre_visibility_operation_fails() {
        for failing_operation in [
            Operation::Position(-2560, -180),
            Operation::Size(2560, 1440),
            Operation::IgnoreCursor,
            Operation::AlwaysOnTop,
            Operation::Focusable,
        ] {
            let window = FakeWindow::new(Some(failing_operation));
            assert!(apply_overlay_window_state(&window, &geometry(), true).is_err());
            assert!(!window.operations().contains(&Operation::Show));
        }
    }

    #[test]
    fn retries_failed_geometry_or_pointer_safety_and_can_recover_visibility() {
        for failing_operation in [Operation::Position(-2560, -180), Operation::IgnoreCursor] {
            let window = FakeWindow::new(Some(failing_operation));
            assert!(apply_overlay_window_state(&window, &geometry(), true).is_err());

            apply_overlay_window_state(&window, &geometry(), true).unwrap();

            assert!(window.operations().ends_with(&safe_show_sequence()));
            assert_eq!(
                window
                    .operations()
                    .iter()
                    .filter(|operation| **operation == Operation::Show)
                    .count(),
                1
            );
        }
    }
}
