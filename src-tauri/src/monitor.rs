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

#[derive(Debug, PartialEq)]
pub struct OverlayGeometry {
    pub position: PhysicalPoint,
    pub physical_width: u32,
    pub physical_height: u32,
    pub logical_width: f64,
    pub logical_height: f64,
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

pub fn full_screen_overlay_geometry(monitor: &MonitorGeometry) -> Option<OverlayGeometry> {
    (monitor.scale_factor.is_finite() && monitor.scale_factor > 0.0).then(|| OverlayGeometry {
        position: PhysicalPoint {
            x: monitor.x,
            y: monitor.y,
        },
        physical_width: monitor.width,
        physical_height: monitor.height,
        logical_width: f64::from(monitor.width) / monitor.scale_factor,
        logical_height: f64::from(monitor.height) / monitor.scale_factor,
    })
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

pub fn logical_short_edge(monitor: &MonitorGeometry) -> Option<f64> {
    (monitor.scale_factor.is_finite() && monitor.scale_factor > 0.0)
        .then(|| f64::from(monitor.width.min(monitor.height)) / monitor.scale_factor)
}

pub fn resolved_logical_short_edge(
    monitors: &[MonitorGeometry],
    target_monitor_id: Option<&str>,
) -> Option<f64> {
    if monitors.is_empty() {
        return None;
    }
    logical_short_edge(resolve_monitor(monitors, target_monitor_id).monitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(
        id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        scale_factor: f64,
        primary: bool,
    ) -> MonitorGeometry {
        MonitorGeometry {
            id: id.into(),
            name: id.into(),
            x,
            y,
            width,
            height,
            scale_factor,
            primary,
        }
    }

    #[test]
    fn full_screen_overlay_uses_physical_origin_and_logical_size() {
        let monitor = monitor("left", -2560, -180, 2560, 1440, 2.0, false);
        assert_eq!(
            full_screen_overlay_geometry(&monitor),
            Some(OverlayGeometry {
                position: PhysicalPoint { x: -2560, y: -180 },
                physical_width: 2560,
                physical_height: 1440,
                logical_width: 1280.0,
                logical_height: 720.0,
            })
        );
    }

    #[test]
    fn full_screen_overlay_rejects_invalid_scale_factor() {
        let monitor = monitor("invalid", 0, 0, 1920, 1080, 0.0, true);
        assert_eq!(full_screen_overlay_geometry(&monitor), None);
    }

    #[test]
    fn full_screen_overlay_preserves_portrait_dimensions() {
        let monitor = monitor("portrait", 1920, 0, 1080, 1920, 1.25, false);
        let geometry = full_screen_overlay_geometry(&monitor).unwrap();
        assert_eq!(geometry.physical_width, 1080);
        assert_eq!(geometry.physical_height, 1920);
        assert_eq!(geometry.logical_width, 864.0);
        assert_eq!(geometry.logical_height, 1536.0);
    }

    #[test]
    fn unavailable_target_falls_back_to_primary_without_losing_preference() {
        let monitors = vec![
            monitor("primary", 0, 0, 1920, 1440, 1.0, true),
            monitor("right", 1920, 0, 2560, 1440, 1.25, false),
        ];

        let resolved = resolve_monitor(&monitors, Some("missing"));

        assert_eq!(resolved.monitor.id, "primary");
        assert!(resolved.using_fallback);
    }

    #[test]
    fn available_target_is_used_even_when_not_primary() {
        let monitors = vec![
            monitor("primary", 0, 0, 1920, 1440, 1.0, true),
            monitor("right", 1920, 0, 2560, 1440, 1.25, false),
        ];

        let resolved = resolve_monitor(&monitors, Some("right"));

        assert_eq!(resolved.monitor.id, "right");
        assert!(!resolved.using_fallback);
    }

    #[test]
    fn logical_short_edge_accounts_for_scale_factor() {
        let monitor = monitor("retina", 0, 0, 2560, 1440, 2.0, true);
        assert_eq!(logical_short_edge(&monitor), Some(720.0));
    }

    #[test]
    fn resolved_short_edge_uses_target_fallback_and_empty_default() {
        let monitors = vec![
            monitor("primary", 0, 0, 1920, 1440, 1.0, true),
            monitor("secondary", 1920, 0, 2560, 1440, 2.0, false),
        ];
        assert_eq!(
            resolved_logical_short_edge(&monitors, Some("secondary")),
            Some(720.0)
        );
        assert_eq!(
            resolved_logical_short_edge(&monitors, Some("missing")),
            Some(1440.0)
        );
        assert_eq!(resolved_logical_short_edge(&[], None), None);
    }
}
