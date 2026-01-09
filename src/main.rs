mod config;
mod monitor;
mod templates;
mod window_manager;
mod cache;
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            println!("{}", "Inicializando configuración...".green().bold());
            config::init_config()?;
            println!("{}", "✓ Configuración creada exitosamente".green());
        }
        Some(Commands::Check) => {
            check_configuration()?;
        }
        Some(Commands::Launch { force_update, verbose }) => {
            launch_waybar(force_update, verbose)?;
        }
        Some(Commands::Monitors) => {
            show_monitors()?;
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
    println!("{}", "  Checking configuration         ".cyan());
    println!();

    let mut cfg = config::load_config()?;
    let wm = window_manager::detect_window_manager()?;
    let connected = monitor::get_connected_monitors(&wm)?;

    println!("{}", "Current configuration:".yellow().bold());
    println!("  Preferred monitor: {}", cfg.display.preferred_monitor.cyan());
    println!("  Monitors configured:");
    for mon in &cfg.display.available_monitors {
        println!("    {} {}", "◆".magenta(), mon);
    }
    println!();

    println!("{}", "System status:".yellow().bold());
    println!("  Window Manager: {}", format!("{:?}", wm).green());
    println!("  Connected monitors:");
    for mon in &connected {
        println!("    {} {}", "●".green(), mon);
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

    // Check for differences and offer to synchronize
    let needs_update = !monitor::lists_match(&cfg.display.available_monitors, &connected);
    
    if needs_update {
        println!("{}", "─────────────────────────────────".yellow());
        println!("{}", "║  ⚠  Differences were detected  ".yellow());
        println!();
        
        if ask_update_config_sync()? {
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

    // 1. Detect window manager
    let wm = window_manager::detect_window_manager()?;
    println!("{} Window manager detected: {}", "✓".green(), format!("{:?}", wm).cyan());

    // 2. Get connected monitors
    let connected = monitor::get_connected_monitors(&wm)?;
    println!("{} Monitors detected: {}", "✓".green(), connected.len().to_string().cyan());
    println!();

    // 3. Load configuration
    let mut cfg = config::load_config()?;

    
    for mon in &cfg.display.available_monitors {
        println!("--CONFIGURATION  {} {}", "◆".magenta(), mon);
    }

    // 4. Show detailed information
    print_monitor_info(&cfg, &connected);

    // 5. Check if an update is needed
    let mut needs_update = !monitor::lists_match(&cfg.display.available_monitors, &connected);

    if needs_update {
        if force_update || ask_update_config()? {
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

    // 6. Verify cache and decide whether to regenerate
    let template_path = templates::get_templates_path(&wm);
    let template_content = fs::read_to_string(&template_path)
        .context("Error reading template file")?;
    let template_hash = cache::calculate_template_hash(&template_content);
    
    let cache_entry = cache::load_cache()?;
    let generated_files_exist = cache::check_generated_files_exist(&connected, &wm);
    
    let should_regenerate = cache::should_regenerate(
        cache_entry.as_ref(),
        &template_hash,
        &connected,
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
            monitors: connected.clone(),
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
        println!("{} The settings are now up to date, using cache.", "✓".green());
        
        if let Some(cache) = cache_entry {
            if verbose {
                use chrono::{DateTime, Utc, TimeZone};
                let dt: DateTime<Utc> = Utc.timestamp_opt(cache.timestamp as i64, 0).unwrap();
                println!("  Latest generation: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
            }
        }
        println!();
    }

    // 7. Cerrar waybar existente
    // monitor::kill_waybar()?;
    if monitor::is_waybar_running() {
        println!("{}", "Closing existing waybar ..".yellow());
        monitor::kill_waybar()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }else{
        println!("{}", "continue because Waybar is not present ..".yellow());
    }

    // 8. Lanzar waybar
    println!();
    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "- INITIALIZING WAYBAR ..         ".cyan());
    println!();

    templates::launch_waybar_instances(&cfg, &connected, &wm, verbose)?;

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
            println!("  {} {}", "◆".magenta(), mon);
        }
    }
    println!();

    println!("{}", "─────────────────────────────────".cyan());
    println!("{}", "MONITORS CONNECTED (detected by the script)".cyan());

    for mon in connected {
        println!("  {} {}", "●".green(), mon);
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

    println!("{} Preferred monitor (configuration): {}", "✓".green(), cfg.display.preferred_monitor.cyan());
    println!();
}

fn ask_update_config() -> Result<bool> {
    println!("{}", "Differences were detected in the monitors".yellow());
    println!();

    println!("{}", "¿Do you want to update the configuration with the detected monitors?".cyan());
    println!("{}", "This will update 'available_monitors' in the TOML file.".cyan());
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

// Version without timeout for when run manually
fn ask_update_config_sync() -> Result<bool> {
    println!("{}", "¿Do you want to synchronize the settings with the detected monitors?".cyan());
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
        .body("Hay diferencias de configuración. Ejecuta 'waybar-manager check' desde la terminal para sincronizar cambios.")
        .icon("dialog-warning")
        .timeout(8000) // 8 segundos
        .show()
        .context("Error al enviar notificación")?;
    
    Ok(())
}

