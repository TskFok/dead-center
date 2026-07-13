use serde::{Deserialize, Serialize};

pub const SETTINGS_VERSION: u8 = 2;
pub const DEFAULT_SIZE_PERCENT: f64 = 3.0;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrosshairPreset {
    DotRing,
    ClassicCross,
    SoftTarget,
    FineDiamond,
    InwardDiamond,
    LongDiamond,
    SolidDiamond,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualSettings {
    pub preset: CrosshairPreset,
    pub primary_color: String,
    pub accent_color: String,
    pub opacity: f64,
    pub size_percent: f64,
    pub stroke_px: f64,
    pub gap_px: f64,
}

impl VisualSettings {
    pub fn normalize(&mut self) {
        self.opacity = self.opacity.clamp(0.1, 1.0);
        self.size_percent = self.size_percent.clamp(0.0, 100.0);
        self.stroke_px = self.stroke_px.clamp(1.0, 8.0);
        self.gap_px = self.gap_px.clamp(0.0, 24.0);

        if !is_hex_color(&self.primary_color) {
            self.primary_color = "#4DFFB8".into();
        } else {
            self.primary_color = self.primary_color.to_uppercase();
        }
        if !is_hex_color(&self.accent_color) {
            self.accent_color = "#F4FF4D".into();
        } else {
            self.accent_color = self.accent_color.to_uppercase();
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacyVisualSettingsV1 {
    pub(crate) preset: CrosshairPreset,
    pub(crate) primary_color: String,
    pub(crate) accent_color: String,
    pub(crate) opacity: f64,
    pub(crate) size_px: f64,
    pub(crate) stroke_px: f64,
    pub(crate) gap_px: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacyAppSettingsV1 {
    pub(crate) version: u8,
    pub(crate) visual: LegacyVisualSettingsV1,
    pub(crate) target_monitor_id: Option<String>,
    pub(crate) toggle_shortcut: String,
    pub(crate) launch_at_login: bool,
    pub(crate) show_on_launch: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub version: u8,
    pub visual: VisualSettings,
    pub target_monitor_id: Option<String>,
    pub toggle_shortcut: String,
    pub launch_at_login: bool,
    pub show_on_launch: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            visual: VisualSettings {
                preset: CrosshairPreset::ClassicCross,
                primary_color: "#4DFFB8".into(),
                accent_color: "#F4FF4D".into(),
                opacity: 0.8,
                size_percent: DEFAULT_SIZE_PERCENT,
                stroke_px: 3.0,
                gap_px: 8.0,
            },
            target_monitor_id: None,
            toggle_shortcut: "Alt+Shift+X".into(),
            launch_at_login: false,
            show_on_launch: true,
        }
    }
}

pub(crate) fn migrate_v1_settings(
    legacy: LegacyAppSettingsV1,
    logical_short_edge: Option<f64>,
) -> AppSettings {
    debug_assert_eq!(legacy.version, 1);
    let size_percent = logical_short_edge
        .filter(|edge| edge.is_finite() && *edge > 0.0)
        .map(|edge| legacy.visual.size_px / edge * 100.0)
        .unwrap_or(DEFAULT_SIZE_PERCENT);
    let mut visual = VisualSettings {
        preset: legacy.visual.preset,
        primary_color: legacy.visual.primary_color,
        accent_color: legacy.visual.accent_color,
        opacity: legacy.visual.opacity,
        size_percent,
        stroke_px: legacy.visual.stroke_px,
        gap_px: legacy.visual.gap_px,
    };
    visual.normalize();
    AppSettings {
        version: SETTINGS_VERSION,
        visual,
        target_monitor_id: legacy.target_monitor_id,
        toggle_shortcut: legacy.toggle_shortcut,
        launch_at_login: legacy.launch_at_login,
        show_on_launch: legacy.show_on_launch,
    }
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_defaults() -> LegacyAppSettingsV1 {
        LegacyAppSettingsV1 {
            version: 1,
            visual: LegacyVisualSettingsV1 {
                preset: CrosshairPreset::ClassicCross,
                primary_color: "#4DFFB8".into(),
                accent_color: "#F4FF4D".into(),
                opacity: 0.8,
                size_px: 32.0,
                stroke_px: 3.0,
                gap_px: 8.0,
            },
            target_monitor_id: None,
            toggle_shortcut: "Alt+Shift+X".into(),
            launch_at_login: false,
            show_on_launch: true,
        }
    }

    #[test]
    fn default_and_serialized_settings_use_percent_size() {
        let settings = AppSettings::default();
        assert_eq!(settings.version, 2);
        assert_eq!(settings.visual.size_percent, 3.0);
        let value = serde_json::to_value(&settings.visual).unwrap();
        assert_eq!(value["sizePercent"], 3.0);
        assert!(value.get("sizePx").is_none());
    }

    #[test]
    fn diamond_presets_use_kebab_case() {
        let cases = [
            (CrosshairPreset::FineDiamond, "\"fine-diamond\""),
            (CrosshairPreset::InwardDiamond, "\"inward-diamond\""),
            (CrosshairPreset::LongDiamond, "\"long-diamond\""),
            (CrosshairPreset::SolidDiamond, "\"solid-diamond\""),
        ];

        for (preset, serialized) in cases {
            assert_eq!(serde_json::to_string(&preset).unwrap(), serialized);
            assert_eq!(
                serde_json::from_str::<CrosshairPreset>(serialized).unwrap(),
                preset
            );
        }
    }

    #[test]
    fn validation_clamps_percent_size() {
        let mut visual = AppSettings::default().visual;
        visual.size_percent = 120.0;
        visual.normalize();
        assert_eq!(visual.size_percent, 100.0);
        visual.size_percent = -1.0;
        visual.normalize();
        assert_eq!(visual.size_percent, 0.0);
    }

    #[test]
    fn migrates_pixels_against_logical_short_edge_and_preserves_fields() {
        let mut legacy = legacy_defaults();
        legacy.visual.preset = CrosshairPreset::FineDiamond;
        legacy.target_monitor_id = Some("secondary".into());
        legacy.toggle_shortcut = "Alt+Shift+Y".into();
        legacy.launch_at_login = true;
        legacy.show_on_launch = false;
        let settings = migrate_v1_settings(legacy, Some(800.0));
        assert_eq!(settings.version, 2);
        assert_eq!(settings.visual.size_percent, 4.0);
        assert_eq!(settings.visual.preset, CrosshairPreset::FineDiamond);
        assert_eq!(settings.target_monitor_id.as_deref(), Some("secondary"));
        assert_eq!(settings.toggle_shortcut, "Alt+Shift+Y");
        assert!(settings.launch_at_login);
        assert!(!settings.show_on_launch);
    }

    #[test]
    fn migration_without_monitor_uses_three_percent() {
        assert_eq!(
            migrate_v1_settings(legacy_defaults(), None)
                .visual
                .size_percent,
            3.0
        );
    }

    #[test]
    fn migration_clamps_converted_percent() {
        let mut legacy = legacy_defaults();
        legacy.visual.size_px = 2000.0;
        assert_eq!(
            migrate_v1_settings(legacy, Some(800.0)).visual.size_percent,
            100.0
        );
    }
}
