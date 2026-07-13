use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrosshairPreset {
    DotRing,
    ClassicCross,
    SoftTarget,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualSettings {
    pub preset: CrosshairPreset,
    pub primary_color: String,
    pub accent_color: String,
    pub opacity: f64,
    pub size_px: f64,
    pub stroke_px: f64,
    pub gap_px: f64,
}

impl VisualSettings {
    pub fn normalize(&mut self) {
        self.opacity = self.opacity.clamp(0.1, 1.0);
        self.size_px = self.size_px.clamp(12.0, 96.0);
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
            version: 1,
            visual: VisualSettings {
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

    #[test]
    fn default_settings_match_confirmed_product_defaults() {
        let settings = AppSettings::default();

        assert_eq!(settings.visual.preset, CrosshairPreset::ClassicCross);
        assert_eq!(settings.visual.primary_color, "#4DFFB8");
        assert_eq!(settings.visual.accent_color, "#F4FF4D");
        assert_eq!(settings.visual.opacity, 0.8);
        assert_eq!(settings.visual.size_px, 32.0);
        assert_eq!(settings.toggle_shortcut, "Alt+Shift+X");
        assert!(settings.show_on_launch);
        assert!(!settings.launch_at_login);
    }

    #[test]
    fn validation_clamps_visual_ranges() {
        let mut visual = VisualSettings {
            opacity: 0.0,
            size_px: 120.0,
            stroke_px: 0.0,
            gap_px: 30.0,
            ..AppSettings::default().visual
        };

        visual.normalize();

        assert_eq!(visual.opacity, 0.1);
        assert_eq!(visual.size_px, 96.0);
        assert_eq!(visual.stroke_px, 1.0);
        assert_eq!(visual.gap_px, 24.0);
    }
}
