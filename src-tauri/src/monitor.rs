use serde::Serialize;
use tauri::{AppHandle, Monitor, Runtime};

#[derive(Clone, Debug)]
pub struct MonitorGeometry {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub primary: bool,
}

#[derive(Debug, PartialEq)]
pub struct PhysicalPoint {
    pub x: i32,
    pub y: i32,
}

pub struct ResolvedMonitor<'a> {
    pub monitor: &'a MonitorGeometry,
    pub using_fallback: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub is_primary: bool,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

pub fn collect_monitors<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<MonitorGeometry>, String> {
    let primary_id = app
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .as_ref()
        .map(monitor_id);

    app.available_monitors()
        .map_err(|error| error.to_string())
        .map(|monitors| {
            monitors
                .into_iter()
                .enumerate()
                .map(|(index, monitor)| {
                    let id = monitor_id(&monitor);
                    let name = monitor
                        .name()
                        .cloned()
                        .unwrap_or_else(|| format!("显示器 {}", index + 1));
                    MonitorGeometry {
                        primary: primary_id.as_ref() == Some(&id),
                        id,
                        name,
                        x: monitor.position().x,
                        y: monitor.position().y,
                        width: monitor.size().width,
                        height: monitor.size().height,
                        scale_factor: monitor.scale_factor(),
                    }
                })
                .collect()
        })
}

pub fn monitor_infos(monitors: &[MonitorGeometry]) -> Vec<MonitorInfo> {
    monitors
        .iter()
        .map(|monitor| MonitorInfo {
            id: monitor.id.clone(),
            name: monitor.name.clone(),
            is_primary: monitor.primary,
            width: monitor.width,
            height: monitor.height,
            scale_factor: monitor.scale_factor,
        })
        .collect()
}

fn monitor_id(monitor: &Monitor) -> String {
    format!(
        "{}@{},{}:{}x{}",
        monitor.name().map(String::as_str).unwrap_or("display"),
        monitor.position().x,
        monitor.position().y,
        monitor.size().width,
        monitor.size().height
    )
}

pub fn centered_overlay_position(
    monitor: &MonitorGeometry,
    overlay_logical_size: f64,
) -> PhysicalPoint {
    let overlay_physical_size = overlay_logical_size * monitor.scale_factor;
    PhysicalPoint {
        x: monitor.x + ((monitor.width as f64 - overlay_physical_size) / 2.0).round() as i32,
        y: monitor.y + ((monitor.height as f64 - overlay_physical_size) / 2.0).round() as i32,
    }
}

pub fn resolve_monitor<'a>(
    monitors: &'a [MonitorGeometry],
    target_id: Option<&str>,
) -> ResolvedMonitor<'a> {
    if let Some(target_id) = target_id {
        if let Some(monitor) = monitors.iter().find(|monitor| monitor.id == target_id) {
            return ResolvedMonitor {
                monitor,
                using_fallback: false,
            };
        }
    }

    let monitor = monitors
        .iter()
        .find(|monitor| monitor.primary)
        .or_else(|| monitors.first())
        .expect("at least one monitor is required");

    ResolvedMonitor {
        monitor,
        using_fallback: target_id.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(id: &str, x: i32, width: u32, scale_factor: f64, primary: bool) -> MonitorGeometry {
        MonitorGeometry {
            id: id.into(),
            name: id.into(),
            x,
            y: 0,
            width,
            height: 1440,
            scale_factor,
            primary,
        }
    }

    #[test]
    fn centers_logical_overlay_on_negative_coordinate_hidpi_monitor() {
        let monitor = monitor("left", -2560, 2560, 2.0, false);

        let point = centered_overlay_position(&monitor, 128.0);

        assert_eq!(point, PhysicalPoint { x: -1408, y: 592 });
    }

    #[test]
    fn unavailable_target_falls_back_to_primary_without_losing_preference() {
        let monitors = vec![
            monitor("primary", 0, 1920, 1.0, true),
            monitor("right", 1920, 2560, 1.25, false),
        ];

        let resolved = resolve_monitor(&monitors, Some("missing"));

        assert_eq!(resolved.monitor.id, "primary");
        assert!(resolved.using_fallback);
    }

    #[test]
    fn available_target_is_used_even_when_not_primary() {
        let monitors = vec![
            monitor("primary", 0, 1920, 1.0, true),
            monitor("right", 1920, 2560, 1.25, false),
        ];

        let resolved = resolve_monitor(&monitors, Some("right"));

        assert_eq!(resolved.monitor.id, "right");
        assert!(!resolved.using_fallback);
    }
}
