use anyhow::{anyhow, Result};
use std::env;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowManager {
    Hyprland,
    Mango,
    Niri,
}

impl WindowManager {
    pub fn as_str(&self) -> &'static str {
        match self {
            WindowManager::Hyprland => "hyprland",
            WindowManager::Mango => "mango",
            WindowManager::Niri => "niri",
        }
    }
}

pub fn detect_window_manager() -> Result<WindowManager> {
    // Detect Hyprland by environment variable
    if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return Ok(WindowManager::Hyprland);
    }

    // Detect Mango by process
    if is_process_running("mango") {
        return Ok(WindowManager::Mango);
    }

    // Detect Niri by process
    if is_process_running("niri") {
        return Ok(WindowManager::Niri);
    }

    Err(anyhow!("No compatible window manager was detected (Hyprland, Mango, Niri)"))
}

fn is_process_running(process_name: &str) -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg(process_name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
