use crate::window_manager::WindowManager;
use anyhow::{Context, Result, anyhow};
use regex::Regex;
use std::collections::HashSet;
use std::process::Command;

pub fn get_connected_monitors(wm: &WindowManager) -> Result<Vec<String>> {
    let output = match wm {
        WindowManager::Hyprland => {
            let output = Command::new("hyprctl")
                .arg("monitors")
                .output()
                .context("Error running hyprctl monitors")?;

            String::from_utf8(output.stdout).context("Error decoding the output of hyprctl")?
        }
        WindowManager::Mango => {
            let output = Command::new("mmsg")
                .arg("-g")
                .output()
                .context("Error executing mmsg -g")?;

            String::from_utf8(output.stdout).context("Error decoding mmsg output")?
        }
        WindowManager::Niri => {
            let output = Command::new("niri")
                .args(["msg", "outputs"])
                .output()
                .context("Error executing niri msg outputs")?;

            String::from_utf8(output.stdout).context("Error decoding niri output")?
        }
    };

    parse_monitors(wm, &output)
}

fn parse_monitors(wm: &WindowManager, output: &str) -> Result<Vec<String>> {
    let mut monitors = Vec::new();

    match wm {
        WindowManager::Hyprland => {
            // Search for lines that begin with "Monitor"
            let re = Regex::new(r"^Monitor\s+(\S+)").unwrap();
            for line in output.lines() {
                if let Some(caps) = re.captures(line) {
                    monitors.push(caps[1].to_string());
                }
            }
        }
        WindowManager::Mango => {
            // Search for lines containing "selmon"
            for line in output.lines() {
                if line.contains("selmon") {
                    if let Some(monitor) = line.split_whitespace().next() {
                        monitors.push(monitor.to_string());
                    }
                }
            }
        }
        WindowManager::Niri => {
            // Search for the monitor in parentheses on lines that begin with "Output"
            let re = Regex::new(r#"^Output\s+"[^"]*"\s+\(([^)]+)\)"#).unwrap();
            for line in output.lines() {
                if let Some(caps) = re.captures(line) {
                    monitors.push(caps[1].to_string());
                }
            }
        }
    }

    if monitors.is_empty() {
        return Err(anyhow!("No monitors were detected"));
    }

    Ok(monitors)
}

pub fn find_matches(configured: &[String], connected: &[String]) -> Vec<String> {
    let configured_set: HashSet<_> = configured.iter().collect();
    let connected_set: HashSet<_> = connected.iter().collect();

    configured_set
        .intersection(&connected_set)
        .map(|s| (*s).clone())
        .collect()
}

pub fn lists_match(list1: &[String], list2: &[String]) -> bool {
    if list1.len() != list2.len() {
        return false;
    }

    let set1: HashSet<_> = list1.iter().collect();
    let set2: HashSet<_> = list2.iter().collect();

    set1 == set2
}

pub fn is_waybar_running() -> bool {
    match Command::new("pgrep").arg("waybar").output() {
        Ok(output) => {
            if !output.status.success() {
                return false;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let pids: Vec<&str> = stdout.trim().lines().collect();

            // Obtain the PID of the current process
            let current_pid = std::process::id();

            // Filter PIDs, excluding the current process
            let waybar_pids: Vec<&str> = pids
                .into_iter()
                .filter(|pid| {
                    if let Ok(pid_num) = pid.parse::<u32>() {
                        pid_num != current_pid
                    } else {
                        true
                    }
                })
                .collect();

            !waybar_pids.is_empty()
        }
        Err(_) => false,
    }
}

pub fn kill_waybar() -> Result<()> {
    // Get the PIDs from Waybar
    let output = Command::new("pidof")
        .arg("waybar")
        .output()
        .context("Error retrieving PIDs from Waybar")?;

    if !output.status.success() || output.stdout.is_empty() {
        // There are no waybar processes running.
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<&str> = stdout.trim().split_whitespace().collect();

    // Get the PID of the current process (waybar-manager)
    let current_pid = std::process::id();

    // Filter and kill only the PIDs that are NOT the current process
    for pid_str in pids {
        if let Ok(pid_num) = pid_str.parse::<u32>() {
            // Do not kill the current process or its direct parents/children
            if pid_num != current_pid {
                Command::new("kill").arg(pid_str).output().ok(); // Ignoring individual mistakes
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hyprland_monitors() {
        let output = r#"Monitor eDP-1 (ID 0):
	1366x768@60.00500 at 1366x0
Monitor HDMI-A-1 (ID 1):
	1920x1080@60.00 at 0x0"#;

        let monitors = parse_monitors(&WindowManager::Hyprland, output).unwrap();
        assert_eq!(monitors, vec!["eDP-1", "HDMI-A-1"]);
    }

    #[test]
    fn test_find_matches() {
        let configured = vec!["eDP-1".to_string(), "HDMI-1".to_string()];
        let connected = vec!["eDP-1".to_string(), "HDMI-A-1".to_string()];

        let matches = find_matches(&configured, &connected);
        assert_eq!(matches.len(), 1);
        assert!(matches.contains(&"eDP-1".to_string()));
    }

    #[test]
    fn test_lists_match() {
        let list1 = vec!["eDP-1".to_string(), "HDMI-A-1".to_string()];
        let list2 = vec!["HDMI-A-1".to_string(), "eDP-1".to_string()];

        assert!(lists_match(&list1, &list2));

        let list3 = vec!["eDP-1".to_string()];
        assert!(!lists_match(&list1, &list3));
    }
}
