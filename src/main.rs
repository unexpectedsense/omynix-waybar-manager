mod cache;
mod config;
mod monitor;
mod templates;
mod window_manager;
use std::fs;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::io::{self, Write};

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "waybar-manager",
    about = "Waybar Manager - Intelligent waybar manager for multiple monitors and Windows Manager - Niri,
    Hyprland and MangoWc",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize configuration with default values
    Init,
    /// Check current configuration
    Check,
    /// Launch waybar on detected monitors
    Launch {
        /// Force configuration update without asking
        #[arg(short, long)]
        force_update: bool,
        /// Verbose mode for debugging
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show detected monitors
    Monitors,
    /// Configure monitors and behavior interactively
    Config,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            println!("{}", "Initializing configuration...".green().bold());
            config::init_config()?;
            println!("{}", "✓ Configuration created successfully".green());
        }
        Some(Commands::Check) => {
            check_configuration()?;
        }
        Some(Commands::Launch {
            force_update,
            verbose,
        }) => {
            launch_waybar(force_update, verbose)?;
        }
        Some(Commands::Monitors) => {
            show_monitors()?;
        }
        Some(Commands::Config) => {
            interactive_config()?;
        }
        None => {
            // Default behavior: launch waybar
            launch_waybar(false, false)?;
        }
    }

    Ok(())
}

fn check_configuration() -> Result<()> {
    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "Checking configuration".cyan());
    println!();

    let mut cfg = config::load_config()?;
    let wm = window_manager::detect_window_manager()?;
    let connected = monitor::get_connected_monitors(&wm)?;

    println!("{}", "Current configuration:".yellow().bold());
    println!(
        "  Preferred monitor: {}",
        cfg.display.preferred_monitor.cyan()
    );
    println!("  Monitors configured:");
    for mon in &cfg.display.available_monitors {
        println!("    {} {}", "-".magenta(), mon);
    }
    println!();

    println!("{}", "System status:".yellow().bold());
    println!("  Window Manager: {}", format!("{:?}", wm).green());
    println!("  Connected monitors:");
    for mon in &connected {
        println!("    {} {}", "-".green(), mon);
    }
    println!();

    let matches = monitor::find_matches(&cfg.display.available_monitors, &connected);
    println!("{}", "Coincidences:".yellow().bold());
    if matches.is_empty() {
        println!("  {} There are no coincidences.", "⚠".yellow());
    } else {
        for mon in &matches {
            println!("    {} {}", "✓".green(), mon);
        }
    }
    println!();

    if cfg.display.mode == "single" {
        println!("Mode: {}", "Single Monitor".cyan());
    } else {
        println!("Mode: {}", "Multiple Monitors".cyan());
    }

    println!();

    // Check for differences and offer to synchronize
    let needs_update = if cfg.display.mode == "single" {
        // In single mode, just verify that your preferred monitor is connected.
        !connected.contains(&cfg.display.preferred_monitor)
    } else {
        // In multiple mode, verify that the lists match.
        !monitor::lists_match(&cfg.display.available_monitors, &connected)
    };

    if needs_update {
        println!("{}", "─────────────────────────────────".yellow());
        println!("{}", "║  ⚠  Differences were detected  ".yellow());
        println!();

        if cfg.display.mode == "single" {
            println!("{}", "⚠ In 'single' mode, you must have the 'preferred_monitor' option configured to disable this alert. The first monitor detected will be used.".yellow());
            println!(
                "{}",
                "  Run 'omynix-waybar-manager config' to reconfigure".cyan()
            );
            println!();
        } else if ask_update_config_sync()? {
            cfg.display.available_monitors = connected.clone();
            config::save_config(&cfg)?;
            println!("{} Configuration successfully synchronized\n", "✓".green());
        } else {
            println!("{} Outdated configuration\n", "⚠".yellow());
        }
    } else {
        println!("{} The configuration is synchronized\n", "✓".green());
    }

    Ok(())
}

fn show_monitors() -> Result<()> {
    let wm = window_manager::detect_window_manager()?;
    let connected = monitor::get_connected_monitors(&wm)?;

    println!("{}", "Monitors detected:".green().bold());
    for (i, mon) in connected.iter().enumerate() {
        println!("  {}. {}", i + 1, mon.cyan());
    }

    Ok(())
}

fn launch_waybar(force_update: bool, verbose: bool) -> Result<()> {
    println!("{}", "─────────────────────────────────".green());
    println!("{}", "- Starting Waybar setup ..    ".green());
    println!();

    // Detect window manager
    let wm = window_manager::detect_window_manager()?;
    println!(
        "{} Window manager detected: {}",
        "✓".green(),
        format!("{:?}", wm).cyan()
    );

    // Get connected monitors
    let connected = monitor::get_connected_monitors(&wm)?;
    println!(
        "{} Monitors detected: {}",
        "✓".green(),
        connected.len().to_string().cyan()
    );
    println!();

    // Load configuration
    let mut cfg = config::load_config()?;

    for mon in &cfg.display.available_monitors {
        println!("--CONFIGURATION  {} {}", "-".magenta(), mon);
    }

    // Show detailed information
    print_monitor_info(&cfg, &connected);

    // Check if an update is needed
    let mut needs_update = if cfg.display.mode == "single" {
        // In single mode, just verify that your preferred monitor is connected.
        !connected.contains(&cfg.display.preferred_monitor)
    } else {
        // In multiple mode, verify that the lists match.
        !monitor::lists_match(&cfg.display.available_monitors, &connected)
    };

    if needs_update {
        if cfg.display.mode == "single" {
            println!("{}", "⚠ The configured monitor is not connected".yellow());
            println!(
                "{}",
                "  Run 'omynix-waybar-manager config' to reconfigure".cyan()
            );
            println!();
        } else if force_update || ask_update_config()? {
            cfg.display.available_monitors = connected.clone();
            config::save_config(&cfg)?;
            needs_update = false;
            println!("{} Configuration updated successfully\n", "✓".green());
        } else {
            println!("{} Outdated configuration\n", "⚠".yellow());
        }
    } else if verbose {
        println!("{} The settings are now updated\n", "✓".green());
    }

    let monitors_to_use = if cfg.display.mode == "single" {
        // Single mode: Only use the preferred monitor if it is connected.
        if connected.contains(&cfg.display.preferred_monitor) {
            vec![cfg.display.preferred_monitor.clone()]
        } else {
            println!(
                "{}",
                "⚠ Preferred monitor not available, using the first one detected".yellow()
            );
            vec![connected[0].clone()]
        }
    } else {
        // Multiple mode: Use all connected devices
        connected.clone()
    };

    // Verify cache and decide whether to regenerate
    let template_path = templates::get_templates_path(&wm);
    let template_content =
        fs::read_to_string(&template_path).context("Error reading template file")?;
    let template_hash = cache::calculate_template_hash(&template_content);

    let cache_entry = cache::load_cache()?;
    let generated_files_exist = cache::check_generated_files_exist(&monitors_to_use, &wm);

    let should_regenerate = cache::should_regenerate(
        cache_entry.as_ref(),
        &template_hash,
        &monitors_to_use,
        &cfg.display.preferred_monitor,
        generated_files_exist,
    );

    if should_regenerate {
        println!("{}", "─────────────────────────────────".cyan());
        println!("{}", "GENERATING CONFIGURATIONS        ".cyan());
        println!();

        templates::generate_configs(&cfg, &connected, &wm, verbose)?;

        // Save cache after generating
        let new_cache = cache::CacheEntry {
            template_hash,
            monitors: monitors_to_use.clone(),
            preferred_monitor: cfg.display.preferred_monitor.clone(),
            timestamp: cache::get_current_timestamp(),
        };
        cache::save_cache(&new_cache)?;

        if verbose {
            println!("{} Cache updated", "✓".green());
        }
    } else {
        println!("{}", "─────────────────────────────────".cyan());
        println!("{}", "- USING CACHE CONFIGURATIONS ..  ".cyan());
        println!();
        println!(
            "{} The settings are now up to date, using cache.",
            "✓".green()
        );

        if let Some(cache) = cache_entry {
            if verbose {
                use chrono::{DateTime, TimeZone, Utc};
                let dt: DateTime<Utc> = Utc.timestamp_opt(cache.timestamp as i64, 0).unwrap();
                println!(
                    "  Latest generation: {}",
                    dt.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
        }
        println!();
    }

    // Close existing waybar
    // monitor::kill_waybar()?;
    if monitor::is_waybar_running() {
        println!("{}", "Closing existing waybar ..".yellow());
        monitor::kill_waybar()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    } else {
        println!("{}", "continue because Waybar is not present ..".yellow());
    }

    // Launch waybar
    println!();
    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "- INITIALIZING WAYBAR ..         ".cyan());
    println!();

    if cfg.display.mode == "single" {
        println!(
            "{}",
            format!(
                "Mode: {} (only in {})",
                "Single Monitor".cyan(),
                monitors_to_use[0]
            )
            .dimmed()
        );
    } else {
        println!(
            "{}",
            format!(
                "Mode: {} ({} monitors)",
                "Multiple Monitors".cyan(),
                monitors_to_use.len()
            )
            .dimmed()
        );
    }
    println!();

    templates::launch_waybar_instances(&cfg, &monitors_to_use, &wm, verbose)?;

    println!();
    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "✓ Waybar started successfully    ".green());

    if needs_update {
        send_config_diff_notification()?;
    }

    Ok(())
}

fn print_monitor_info(cfg: &config::Config, connected: &[String]) {
    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "- CONFIGURED MONITORS (from TOML file):".cyan());

    if cfg.display.available_monitors.is_empty() {
        println!("  {}", "(None configured)".yellow());
    } else {
        for mon in &cfg.display.available_monitors {
            println!("  {} {}", "-".magenta(), mon);
        }
    }
    println!();

    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "MONITORS CONNECTED (detected by the script)".cyan());

    for mon in connected {
        println!("  {} {}", "-".green(), mon);
    }
    println!();

    let matches = monitor::find_matches(&cfg.display.available_monitors, connected);
    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "MATCHES (monitors on both lists) ".cyan());

    if matches.is_empty() {
        println!("  {} There are no coincidences.", "⚠".yellow());
    } else {
        for mon in &matches {
            println!("  {} {}", "✓".green(), mon);
        }
    }
    println!();

    println!(
        "{} Preferred monitor (configuration): {}",
        "✓".green(),
        cfg.display.preferred_monitor.cyan()
    );
    println!();
}

fn ask_update_config() -> Result<bool> {
    println!("{}", "Differences were detected in the monitors".yellow());
    println!();

    println!(
        "{}",
        "¿Do you want to update the configuration with the detected monitors?".cyan()
    );
    println!(
        "{}",
        "This will update 'available_monitors' in the TOML file.".cyan()
    );
    println!();

    print!("{}", "Update settings? [y/n] (4 seconds): ".green());
    io::stdout().flush()?;

    // Create a channel for communication between threads
    let (tx, rx) = mpsc::channel();

    // Thread for read input
    thread::spawn(move || {
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            tx.send(input).ok();
        }
    });

    // Wait 4 seconds for a response
    match rx.recv_timeout(Duration::from_secs(4)) {
        Ok(input) => {
            let input = input.trim().to_lowercase();
            Ok(input == "y" || input == "yes")
        }
        Err(_) => {
            // Timeout - no response
            println!("\n{}", "⏱  Time expired. Skipping update.".yellow());
            Ok(false)
        }
    }
}

fn ask_update_config_sync() -> Result<bool> {
    println!(
        "{}",
        "¿Do you want to synchronize the settings with the detected monitors?".cyan()
    );
    println!();

    print!("{}", "Sync now? [Y/n]: ".green());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(input.is_empty() || input == "y" || input == "yes")
}

fn send_config_diff_notification() -> Result<()> {
    use notify_rust::Notification;

    Notification::new()
        .summary("Omynix Waybar Manager")
        .body("There are configuration differences. Run 'waybar-manager check' from the terminal to synchronize changes.")
        .icon("dialog-warning")
        .timeout(8000) // 8 seconds
        .show()
        .context("Error sending notification")?;

    Ok(())
}

fn interactive_config() -> Result<()> {
    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "Interactive Monitor Configuration".cyan());
    println!();

    // Detect window and monitor manager
    let wm = window_manager::detect_window_manager()?;
    let connected = monitor::get_connected_monitors(&wm)?;

    if connected.is_empty() {
        println!("{}", "⚠ No connected monitors were detected".red());
        return Ok(());
    }

    println!("{}", "Monitors detected:".yellow().bold());
    for (i, mon) in connected.iter().enumerate() {
        println!("  {}. {}", i + 1, mon.cyan());
    }
    println!();

    // Ask about operating mode
    println!("{}", "¿How do you want to configure Waybar?".green().bold());
    println!(
        "  1. {} - Single monitor (full setup)",
        "Single Monitor".cyan()
    );
    println!(
        "  2. {} - Multiple monitors (differentiated)",
        "Multiple Monitors".cyan()
    );
    println!();
    print!("{}", "Select an option [1/2]: ".green());
    io::stdout().flush()?;

    let mut mode = String::new();
    io::stdin().read_line(&mut mode)?;
    let mode = mode.trim();

    let mut cfg = config::load_config()?;

    match mode {
        "1" => {
            // Single monitor mode
            configure_single_monitor(&connected, &mut cfg)?;
        }
        "2" => {
            // Multi-monitor mode
            configure_multiple_monitors(&connected, &mut cfg)?;
        }
        _ => {
            println!("{}", "⚠ Opción no válida".yellow());
            return Ok(());
        }
    }

    // Save settings
    config::save_config(&cfg)?;

    println!();
    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "✓ Configuration saved successfully".green());
    println!();
    println!(
        "{}",
        "Run 'waybar-manager launch' to apply the changes.".cyan()
    );

    Ok(())
}

fn configure_single_monitor(connected: &[String], cfg: &mut config::Config) -> Result<()> {
    println!();
    println!("{}", "═══ Mode: Single Monitor ═══".cyan().bold());
    println!();

    if connected.len() == 1 {
        // There's only one monitor, use it automatically
        cfg.display.preferred_monitor = connected[0].clone();
        cfg.display.available_monitors = vec![connected[0].clone()];
        cfg.display.mode = "single".to_string();

        println!(
            "{}",
            format!("✓ Selected monitor: {}", connected[0]).green()
        );
    } else {
        // Multiple monitors detected, choose which one to use
        println!(
            "{}",
            "Select the monitor where you want to run Waybar:".yellow()
        );
        for (i, mon) in connected.iter().enumerate() {
            println!("  {}. {}", i + 1, mon.cyan());
        }
        println!();
        print!("{}", "Monitor number: ".green());
        io::stdout().flush()?;

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;

        if let Ok(idx) = choice.trim().parse::<usize>() {
            if idx > 0 && idx <= connected.len() {
                let selected = &connected[idx - 1];
                cfg.display.preferred_monitor = selected.clone();
                cfg.display.available_monitors = vec![selected.clone()];
                cfg.display.mode = "single".to_string();

                println!();
                println!("{}", format!("✓ Selected monitor: {}", selected).green());
            } else {
                println!("{}", "⚠ Invalid number".yellow());
            }
        } else {
            println!("{}", "⚠ Invalid entry".yellow());
        }
    }

    Ok(())
}

fn configure_multiple_monitors(connected: &[String], cfg: &mut config::Config) -> Result<()> {
    println!();
    println!("{}", "═══ Mode: Multiple Monitors ═══".cyan().bold());
    println!();

    // Select preferred monitor (with FULL settings)
    println!(
        "{}",
        "Select the MAIN monitor (full setup):".yellow().bold()
    );
    for (i, mon) in connected.iter().enumerate() {
        println!("  {}. {}", i + 1, mon.cyan());
    }
    println!();
    print!("{}", "Main monitor number: ".green());
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;

    let preferred_idx = if let Ok(idx) = choice.trim().parse::<usize>() {
        if idx > 0 && idx <= connected.len() {
            idx - 1
        } else {
            println!("{}", "⚠ Invalid number, using the first one".yellow());
            0
        }
    } else {
        println!("{}", "⚠ Invalid input, using the first".yellow());
        0
    };

    cfg.display.preferred_monitor = connected[preferred_idx].clone();
    cfg.display.mode = "multiple".to_string();

    println!();
    println!(
        "{}",
        format!("✓ Preferred monitor: {}", connected[preferred_idx]).green()
    );
    println!();

    // Select additional monitors (with SIMPLE setup)
    println!(
        "{}",
        "Select SECONDARY monitors (simple setup):".yellow().bold()
    );
    println!(
        "{}",
        "Select the monitors you wish to include (separated by commas)".dimmed()
    );

    for (i, mon) in connected.iter().enumerate() {
        if i == preferred_idx {
            println!("  {}. {} {}", i + 1, mon.cyan(), "(main)".dimmed());
        } else {
            println!("  {}. {}", i + 1, mon);
        }
    }
    println!();
    print!(
        "{}",
        "Monitor numbers (ex: 1,2,3) or ENTER for all: ".green()
    );
    io::stdout().flush()?;

    let mut selection = String::new();
    io::stdin().read_line(&mut selection)?;
    let selection = selection.trim();

    if selection.is_empty() {
        // Use all monitors
        cfg.display.available_monitors = connected.to_vec();
        println!();
        println!("{}", "✓ Using all detected monitors".green());
    } else {
        // Parse selection
        let mut selected = Vec::new();
        selected.push(connected[preferred_idx].clone()); // Always include the main one

        for num_str in selection.split(',') {
            if let Ok(idx) = num_str.trim().parse::<usize>() {
                if idx > 0 && idx <= connected.len() {
                    let mon = &connected[idx - 1];
                    if !selected.contains(mon) {
                        selected.push(mon.clone());
                    }
                }
            }
        }

        cfg.display.available_monitors = selected.clone();

        println!();
        println!("{}", "✓ Selected monitors:".green());
        for mon in &selected {
            if mon == &cfg.display.preferred_monitor {
                println!("  • {} {}", mon.cyan(), "(main - FULL)".green());
            } else {
                println!("  • {} {}", mon, "(secondary - SIMPLE)".dimmed());
            }
        }
    }

    Ok(())
}
