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

fn apply_overlay_window_safety<W: OverlayWindowOps>(
    window: &W,
    geometry: &OverlayGeometry,
) -> Result<(), String> {
    window.set_physical_position(geometry.position.x, geometry.position.y)?;
    window.set_physical_size(geometry.physical_width, geometry.physical_height)?;
    window.ignore_cursor_events()?;
    window.keep_always_on_top()?;
    window.disable_focus()
}

trait MainThreadScheduler {
    fn schedule_main_thread<F>(&self, task: F) -> Result<(), String>
    where
        F: FnOnce() + Send + 'static;
}

impl<R: Runtime> MainThreadScheduler for AppHandle<R> {
    fn schedule_main_thread<F>(&self, task: F) -> Result<(), String>
    where
        F: FnOnce() + Send + 'static,
    {
        AppHandle::run_on_main_thread(self, task).map_err(|error| error.to_string())
    }
}

struct ResolvedOverlayGeometry {
    geometry: OverlayGeometry,
    monitor_id: String,
    using_fallback: bool,
}

trait OverlaySyncContext: Send + 'static {
    type Window: OverlayWindowOps;

    fn desired_visibility(&self) -> Result<bool, String>;
    fn resolve_geometry(&self) -> Result<ResolvedOverlayGeometry, String>;
    fn window(&self) -> Option<Self::Window>;
    fn create_window(&self) -> Result<(), String>;
    fn record_overlay_success(&self, resolved: &ResolvedOverlayGeometry) -> Result<(), String>;
    fn record_overlay_failure(&self, error: String);
}

fn combine_hide_failure(error: String, hide_error: String) -> String {
    format!("{error}；同时隐藏覆盖层失败：{hide_error}")
}

fn hide_after_failure<C>(context: &C, error: String) -> String
where
    C: OverlaySyncContext,
{
    match context.window().map(|window| window.hide()) {
        Some(Err(hide_error)) => combine_hide_failure(error, hide_error),
        _ => error,
    }
}

fn execute_overlay_sync<C>(context: &C, create_if_missing: bool) -> Result<(), String>
where
    C: OverlaySyncContext,
{
    let should_show = context
        .desired_visibility()
        .map_err(|error| hide_after_failure(context, error))?;
    if !should_show {
        if let Some(window) = context.window() {
            window
                .hide()
                .map_err(|hide_error| format!("隐藏覆盖层失败：{hide_error}"))?;
        }
    }

    let sync_result = (|| {
        let resolved = context.resolve_geometry()?;

        if create_if_missing && context.window().is_none() {
            context.create_window()?;
        }

        if let Some(window) = context.window() {
            apply_overlay_window_safety(&window, &resolved.geometry)?;
            if should_show {
                window.show()?;
            }
        } else if should_show {
            return Err("覆盖层窗口不存在".into());
        }

        context.record_overlay_success(&resolved)
    })();

    match sync_result {
        Ok(()) => Ok(()),
        Err(error) if should_show => Err(hide_after_failure(context, error)),
        Err(error) => Err(error),
    }
}

fn schedule_overlay_sync<S, C>(
    scheduler: &S,
    context: C,
    create_if_missing: bool,
) -> Result<(), String>
where
    S: MainThreadScheduler,
    C: OverlaySyncContext,
{
    scheduler.schedule_main_thread(move || {
        if let Err(error) = execute_overlay_sync(&context, create_if_missing) {
            context.record_overlay_failure(error);
        }
    })
}

struct AppOverlaySyncContext<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> OverlaySyncContext for AppOverlaySyncContext<R> {
    type Window = WebviewWindow<R>;

    fn desired_visibility(&self) -> Result<bool, String> {
        self.app
            .state::<AppState>()
            .status
            .lock()
            .map_err(|error| error.to_string())
            .map(|status| status.visible)
    }

    fn resolve_geometry(&self) -> Result<ResolvedOverlayGeometry, String> {
        let monitors = collect_monitors(&self.app)?;
        if monitors.is_empty() {
            return Err("没有检测到可用显示器".into());
        }
        let target = self
            .app
            .state::<AppState>()
            .settings
            .lock()
            .map_err(|error| error.to_string())?
            .target_monitor_id
            .clone();
        let resolved = resolve_monitor(&monitors, target.as_deref());
        let geometry = full_screen_overlay_geometry(resolved.monitor)
            .ok_or_else(|| "目标显示器缩放比例无效".to_string())?;
        Ok(ResolvedOverlayGeometry {
            geometry,
            monitor_id: resolved.monitor.id.clone(),
            using_fallback: resolved.using_fallback,
        })
    }

    fn window(&self) -> Option<Self::Window> {
        self.app.get_webview_window(OVERLAY_LABEL)
    }

    fn create_window(&self) -> Result<(), String> {
        create_overlay_window(&self.app)
    }

    fn record_overlay_success(&self, resolved: &ResolvedOverlayGeometry) -> Result<(), String> {
        let app_state = self.app.state::<AppState>();
        let mut status = app_state.status.lock().map_err(|error| error.to_string())?;
        status.resolved_monitor_id = resolved.monitor_id.clone();
        status.using_fallback_monitor = resolved.using_fallback;
        status.error = None;
        let status_payload = status.clone();
        drop(status);
        let _ = self.app.emit("crosshair://runtime-changed", status_payload);
        Ok(())
    }

    fn record_overlay_failure(&self, error: String) {
        record_overlay_error(&self.app, error);
    }
}

fn schedule_app_overlay_sync<R: Runtime>(
    app: &AppHandle<R>,
    create_if_missing: bool,
) -> Result<(), String> {
    schedule_overlay_sync(
        app,
        AppOverlaySyncContext { app: app.clone() },
        create_if_missing,
    )
}

fn create_overlay_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
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
    Ok(())
}

pub fn create_or_refresh_overlay<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    schedule_app_overlay_sync(app, true)
}

pub fn refresh_overlay_geometry<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    schedule_app_overlay_sync(app, false)
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
    let app_state = app.state::<AppState>();
    app_state
        .status
        .lock()
        .map_err(|error| error.to_string())?
        .visible = visible;
    schedule_app_overlay_sync(app, visible)
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
        let status_payload = status.clone();
        drop(status);
        let _ = app.emit("crosshair://runtime-changed", status_payload);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

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
        operations: Mutex<Vec<Operation>>,
        failures: Mutex<Vec<Operation>>,
        visible: Mutex<bool>,
    }

    impl FakeWindow {
        fn new(visible: bool, fail_once: Option<Operation>) -> Self {
            Self::with_failures(visible, fail_once.into_iter())
        }

        fn with_failures(visible: bool, failures: impl IntoIterator<Item = Operation>) -> Self {
            Self {
                operations: Mutex::new(Vec::new()),
                failures: Mutex::new(failures.into_iter().collect()),
                visible: Mutex::new(visible),
            }
        }

        fn perform(&self, operation: Operation) -> Result<(), String> {
            self.operations.lock().unwrap().push(operation);
            let mut failures = self.failures.lock().unwrap();
            if let Some(index) = failures.iter().position(|failure| *failure == operation) {
                failures.remove(index);
                Err("计划内失败".into())
            } else {
                drop(failures);
                match operation {
                    Operation::Show => *self.visible.lock().unwrap() = true,
                    Operation::Hide => *self.visible.lock().unwrap() = false,
                    _ => {}
                }
                Ok(())
            }
        }

        fn operations(&self) -> Vec<Operation> {
            self.operations.lock().unwrap().clone()
        }

        fn is_visible(&self) -> bool {
            *self.visible.lock().unwrap()
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

    impl OverlayWindowOps for Arc<FakeWindow> {
        fn set_physical_position(&self, x: i32, y: i32) -> Result<(), String> {
            self.as_ref().set_physical_position(x, y)
        }

        fn set_physical_size(&self, width: u32, height: u32) -> Result<(), String> {
            self.as_ref().set_physical_size(width, height)
        }

        fn ignore_cursor_events(&self) -> Result<(), String> {
            self.as_ref().ignore_cursor_events()
        }

        fn keep_always_on_top(&self) -> Result<(), String> {
            self.as_ref().keep_always_on_top()
        }

        fn disable_focus(&self) -> Result<(), String> {
            self.as_ref().disable_focus()
        }

        fn show(&self) -> Result<(), String> {
            self.as_ref().show()
        }

        fn hide(&self) -> Result<(), String> {
            self.as_ref().hide()
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

    fn safe_configuration_sequence() -> Vec<Operation> {
        vec![
            Operation::Position(-2560, -180),
            Operation::Size(2560, 1440),
            Operation::IgnoreCursor,
            Operation::AlwaysOnTop,
            Operation::Focusable,
        ]
    }

    #[test]
    fn applies_geometry_and_safety_without_changing_visibility() {
        let visible_window = FakeWindow::new(true, None);
        apply_overlay_window_safety(&visible_window, &geometry()).unwrap();
        assert_eq!(visible_window.operations(), safe_configuration_sequence());
        assert!(visible_window.is_visible());

        let hidden_window = FakeWindow::new(false, None);
        apply_overlay_window_safety(&hidden_window, &geometry()).unwrap();
        assert_eq!(hidden_window.operations(), safe_configuration_sequence());
        assert!(!hidden_window.is_visible());
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
            let window = FakeWindow::new(false, Some(failing_operation));
            assert!(apply_overlay_window_safety(&window, &geometry()).is_err());
            assert!(!window.operations().contains(&Operation::Show));
            assert!(!window.operations().contains(&Operation::Hide));
            assert!(!window.is_visible());
        }
    }

    #[test]
    fn retries_failed_geometry_or_pointer_safety_and_can_recover_visibility() {
        for failing_operation in [Operation::Position(-2560, -180), Operation::IgnoreCursor] {
            let window = FakeWindow::new(false, Some(failing_operation));
            assert!(apply_overlay_window_safety(&window, &geometry()).is_err());

            apply_overlay_window_safety(&window, &geometry()).unwrap();
            window.show().unwrap();

            let mut successful_show = safe_configuration_sequence();
            successful_show.push(Operation::Show);
            assert!(window.operations().ends_with(&successful_show));
            assert_eq!(
                window
                    .operations()
                    .iter()
                    .filter(|operation| **operation == Operation::Show)
                    .count(),
                1
            );
            assert!(window.is_visible());
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct FakeStatus {
        visible: bool,
        error: Option<String>,
    }

    #[derive(Clone)]
    struct FakeOverlayTarget {
        status: Arc<Mutex<FakeStatus>>,
        events: Arc<Mutex<Vec<FakeStatus>>>,
        window: Arc<FakeWindow>,
        geometry_error: Arc<Mutex<Option<String>>>,
    }

    impl FakeOverlayTarget {
        fn new(visible: bool, fail_once: Option<Operation>) -> Self {
            Self::with_failures(visible, fail_once.into_iter())
        }

        fn with_failures(visible: bool, failures: impl IntoIterator<Item = Operation>) -> Self {
            Self {
                status: Arc::new(Mutex::new(FakeStatus {
                    visible,
                    error: None,
                })),
                events: Arc::new(Mutex::new(Vec::new())),
                window: Arc::new(FakeWindow::with_failures(visible, failures)),
                geometry_error: Arc::new(Mutex::new(None)),
            }
        }

        fn set_visible(&self, visible: bool) {
            self.status.lock().unwrap().visible = visible;
        }

        fn status(&self) -> FakeStatus {
            self.status.lock().unwrap().clone()
        }

        fn set_geometry_error(&self, error: Option<&str>) {
            *self.geometry_error.lock().unwrap() = error.map(str::to_owned);
        }

        fn publish_status(&self) {
            self.events.lock().unwrap().push(self.status());
        }
    }

    impl OverlaySyncContext for FakeOverlayTarget {
        type Window = Arc<FakeWindow>;

        fn desired_visibility(&self) -> Result<bool, String> {
            Ok(self.status().visible)
        }

        fn resolve_geometry(&self) -> Result<ResolvedOverlayGeometry, String> {
            if let Some(error) = self.geometry_error.lock().unwrap().clone() {
                return Err(error);
            }
            Ok(ResolvedOverlayGeometry {
                geometry: geometry(),
                monitor_id: "fake".into(),
                using_fallback: false,
            })
        }

        fn window(&self) -> Option<Self::Window> {
            Some(self.window.clone())
        }

        fn create_window(&self) -> Result<(), String> {
            Ok(())
        }

        fn record_overlay_success(
            &self,
            _resolved: &ResolvedOverlayGeometry,
        ) -> Result<(), String> {
            self.status.lock().unwrap().error = None;
            self.publish_status();
            Ok(())
        }

        fn record_overlay_failure(&self, error: String) {
            self.status.lock().unwrap().error = Some(error);
            self.publish_status();
        }
    }

    #[derive(Default)]
    struct ControlledMainThread {
        tasks: Mutex<VecDeque<Box<dyn FnOnce() + Send>>>,
    }

    impl ControlledMainThread {
        fn run_next(&self) {
            self.tasks.lock().unwrap().pop_front().unwrap()();
        }

        fn run_all(&self) {
            while !self.tasks.lock().unwrap().is_empty() {
                self.run_next();
            }
        }
    }

    impl MainThreadScheduler for ControlledMainThread {
        fn schedule_main_thread<F>(&self, task: F) -> Result<(), String>
        where
            F: FnOnce() + Send + 'static,
        {
            self.tasks.lock().unwrap().push_back(Box::new(task));
            Ok(())
        }
    }

    #[test]
    fn scheduled_sync_reads_visibility_when_main_thread_task_executes() {
        let main_thread = ControlledMainThread::default();
        let target = FakeOverlayTarget::new(true, None);

        schedule_overlay_sync(&main_thread, target.clone(), false).unwrap();
        target.set_visible(false);
        main_thread.run_next();

        assert_eq!(target.window.operations().first(), Some(&Operation::Hide));
        assert_eq!(
            target
                .window
                .operations()
                .iter()
                .filter(|operation| **operation == Operation::Hide)
                .count(),
            1
        );
        assert!(!target.window.is_visible());
        assert_eq!(target.events.lock().unwrap().len(), 1);
        assert_eq!(target.status().error, None);
    }

    #[test]
    fn interleaved_poll_and_visibility_sync_finish_at_latest_desired_state() {
        let main_thread = ControlledMainThread::default();
        let target = FakeOverlayTarget::new(true, None);

        schedule_overlay_sync(&main_thread, target.clone(), false).unwrap();
        target.set_visible(false);
        schedule_overlay_sync(&main_thread, target.clone(), false).unwrap();
        main_thread.run_next();
        target.set_visible(true);
        schedule_overlay_sync(&main_thread, target.clone(), true).unwrap();
        main_thread.run_all();

        assert!(target.status().visible);
        assert_eq!(target.window.operations().last(), Some(&Operation::Show));
        assert!(target.window.is_visible());
    }

    #[test]
    fn visible_geometry_failure_hides_existing_window() {
        let main_thread = ControlledMainThread::default();
        let target = FakeOverlayTarget::new(true, None);
        target.set_geometry_error(Some("几何解析失败"));

        schedule_overlay_sync(&main_thread, target.clone(), false).unwrap();
        main_thread.run_next();

        assert!(!target.window.is_visible());
        assert_eq!(target.window.operations(), vec![Operation::Hide]);
        assert_eq!(target.status().error.as_deref(), Some("几何解析失败"));
    }

    #[test]
    fn every_pre_visibility_window_failure_hides_existing_window() {
        for failing_operation in [
            Operation::Position(-2560, -180),
            Operation::Size(2560, 1440),
            Operation::IgnoreCursor,
            Operation::AlwaysOnTop,
            Operation::Focusable,
        ] {
            let main_thread = ControlledMainThread::default();
            let target = FakeOverlayTarget::new(true, Some(failing_operation));

            schedule_overlay_sync(&main_thread, target.clone(), false).unwrap();
            main_thread.run_next();

            assert!(!target.window.is_visible(), "{failing_operation:?}");
            assert!(!target.window.operations().contains(&Operation::Show));
            assert_eq!(target.window.operations().last(), Some(&Operation::Hide));
            assert_eq!(target.status().error.as_deref(), Some("计划内失败"));
            assert_eq!(target.events.lock().unwrap().last(), Some(&target.status()));
        }
    }

    #[test]
    fn hidden_geometry_failure_hides_before_resolving_geometry() {
        let main_thread = ControlledMainThread::default();
        let target = FakeOverlayTarget::new(true, None);
        target.set_visible(false);
        target.set_geometry_error(Some("隐藏请求几何失败"));

        schedule_overlay_sync(&main_thread, target.clone(), false).unwrap();
        main_thread.run_next();

        assert!(!target.window.is_visible());
        assert_eq!(target.window.operations(), vec![Operation::Hide]);
        assert_eq!(target.status().error.as_deref(), Some("隐藏请求几何失败"));
        assert_eq!(target.events.lock().unwrap().len(), 1);
    }

    #[test]
    fn failed_initial_hide_stops_sync_and_emits_only_failure() {
        let main_thread = ControlledMainThread::default();
        let target = FakeOverlayTarget::new(true, Some(Operation::Hide));
        target.set_visible(false);

        schedule_overlay_sync(&main_thread, target.clone(), false).unwrap();
        main_thread.run_next();

        assert_eq!(target.window.operations(), vec![Operation::Hide]);
        assert!(target.window.is_visible());
        assert_eq!(
            target.status().error.as_deref(),
            Some("隐藏覆盖层失败：计划内失败")
        );
        assert_eq!(target.events.lock().unwrap().len(), 1);
    }

    #[test]
    fn failure_keeps_original_error_when_best_effort_hide_also_fails() {
        let main_thread = ControlledMainThread::default();
        let target =
            FakeOverlayTarget::with_failures(true, [Operation::Size(2560, 1440), Operation::Hide]);

        schedule_overlay_sync(&main_thread, target.clone(), false).unwrap();
        main_thread.run_next();

        let error = target.status().error.unwrap();
        assert!(error.starts_with("计划内失败"));
        assert!(error.contains("隐藏覆盖层失败"));
        assert!(target.window.is_visible());
    }

    #[test]
    fn scheduled_window_failure_is_recorded_and_next_sync_recovers_visibility() {
        let main_thread = ControlledMainThread::default();
        let target = FakeOverlayTarget::new(true, Some(Operation::IgnoreCursor));

        assert!(schedule_overlay_sync(&main_thread, target.clone(), false).is_ok());
        main_thread.run_next();

        assert!(!target.window.is_visible());
        assert_eq!(target.status().error.as_deref(), Some("计划内失败"));
        assert_eq!(target.events.lock().unwrap().last(), Some(&target.status()));

        schedule_overlay_sync(&main_thread, target.clone(), false).unwrap();
        main_thread.run_next();
        assert_eq!(target.window.operations().last(), Some(&Operation::Show));
        assert!(target.window.is_visible());
        assert_eq!(target.status().error, None);
    }
}
