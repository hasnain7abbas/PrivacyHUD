use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HudColor {
    Green,
    Red,
    Yellow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub mic_active: bool,
    pub cam_active: bool,
    pub active_processes: Vec<String>,
}

impl DeviceStatus {
    pub fn hud_color(&self) -> HudColor {
        match (self.mic_active, self.cam_active) {
            (false, false) => HudColor::Green,
            (false, true) => HudColor::Yellow,
            _ => HudColor::Red,
        }
    }
}
