use crate::config::Config;
use crate::window_manager::WindowManager;
use anyhow::{Context, Result};
use colored::*;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{PathBuf};
use std::process::Command;


#[derive(Debug)]
pub struct TemplateConfig {
    pub template_type: TemplateType,
    pub config: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateType {
    Full,
    Simple,
    Custom(String),
}

impl TemplateType {
    fn from_comment(comment: &str) -> Option<Self> {
        if comment.contains("TPL:FULL") {
            Some(TemplateType::Full)
        } else if comment.contains("TPL:SIMPLE") {
            Some(TemplateType::Simple)
        } else if let Some(custom) = comment.strip_prefix("TPL:") {
            Some(TemplateType::Custom(custom.trim().to_string()))
        } else {
            None
        }
    }
}

pub fn get_templates_path(wm: &WindowManager) -> PathBuf {
    let home = dirs::home_dir().unwrap();
    home.join(".config/waybar/templates")
        .join(format!("{}.jsonc", wm.as_str()))
}

pub fn get_generated_config_path(wm: &WindowManager, monitor: &str, template_type: &TemplateType) -> PathBuf {
    let home = dirs::home_dir().unwrap();
    let type_str = match template_type {
        TemplateType::Full => "full",
        TemplateType::Simple => "simple",
        TemplateType::Custom(name) => name.as_str(),
    };
    
    home.join(".config/waybar/generated")
        .join(format!("{}_{}_{}. json", wm.as_str(), monitor, type_str))
}

pub fn load_templates(wm: &WindowManager) -> Result<Vec<TemplateConfig>> {
    let template_path = get_templates_path(wm);
    
    println!("Looking for templates in: {}", template_path.display());
    
    if !template_path.exists() {
        return Err(anyhow::anyhow!(
            "No template file was found in: {}",
            template_path.display()
        ));
    }

    let content = fs::read_to_string(&template_path)
        .context("Error reading template file")?;

    println!("File contents (first 200 characters)):\n{}\n", 
             &content.chars().take(200).collect::<String>());

    // Parse JSONC (JSON with comments)
    let configs = parse_jsonc_templates(&content)?;

    Ok(configs)
}

fn parse_jsonc_templates(content: &str) -> Result<Vec<TemplateConfig>> {
    let mut templates = Vec::new();
    
    // Extract template markers
    let mut template_types_in_order = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            if let Some(tpl_type) = TemplateType::from_comment(trimmed) {
                template_types_in_order.push(tpl_type);
            }
        }
    }
    
    // Clear comments
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '/' if chars.peek() == Some(&'/') => {
                // Line comment - skip to the end of the line
                chars.next(); // consume the second '/'
                while let Some(c) = chars.next() {
                    if c == '\n' {
                        result.push('\n');
                        break;
                    }
                }
            }
            '"' => {
                // Within a string - keep everything including possible //
                result.push(ch);
                let mut escaped = false;
                while let Some(c) = chars.next() {
                    result.push(c);
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        break;
                    }
                }
            }
            _ => {
                result.push(ch);
            }
        }
    }
    
    // Parse the clean JSON
    let json_array: Vec<Value> = serde_json::from_str(&result)
        .context(format!(
            "Error parsing template file.\nFirst 300 characters of clean content:\n{}",
            &result.chars().take(300).collect::<String>()
        ))?;
    
    // Assign template types
    for (i, config) in json_array.into_iter().enumerate() {
        let template_type = if i < template_types_in_order.len() {
            template_types_in_order[i].clone()
        } else {
            match i {
                0 => TemplateType::Full,
                1 => TemplateType::Simple,
                _ => TemplateType::Custom(format!("template_{}", i)),
            }
        };
        
        templates.push(TemplateConfig {
            template_type,
            config,
        });
    }
    
    if templates.is_empty() {
        return Err(anyhow::anyhow!("No valid templates were found in the file"));
    }
    
    Ok(templates)
}

pub fn generate_configs(
    cfg: &Config,
    connected: &[String],
    wm: &WindowManager,
    verbose: bool,
) -> Result<()> {
    let templates = load_templates(wm)?;

    if verbose {
        println!("Templates loaded: {}", templates.len());
    }

    // Create directory of generated configs if it does not exist
    let generated_dir = dirs::home_dir()
        .unwrap()
        .join(".config/waybar/generated");
    fs::create_dir_all(&generated_dir)?;

    // Determine which configuration to use for each monitor
    let config_assignments = determine_config_assignments(cfg, connected);

    for (monitor, template_type) in &config_assignments {
        // Find the corresponding template
        let template = templates
            .iter()
            .find(|t| &t.template_type == template_type)
            .context(format!("No template was found for {:?}", template_type))?;

        // Generate configuration with the configured output
        let mut config = template.config.clone();
        if let Some(obj) = config.as_object_mut() {
            obj.insert("output".to_string(), Value::String(monitor.clone()));
        }

        // Save generated settings
        let output_path = get_generated_config_path(wm, monitor, template_type);
        let json_str = serde_json::to_string_pretty(&config)?;
        fs::write(&output_path, json_str)?;

        if verbose {
            println!(
                "  {} Generated: {} → {:?}",
                "✓".green(),
                monitor.cyan(),
                template_type
            );
        }
    }

    Ok(())
}

fn determine_config_assignments(
    cfg: &Config,
    connected: &[String],
) -> HashMap<String, TemplateType> {
    let mut assignments = HashMap::new();

    if connected.len() == 1 {
        // One monitor: always FULL
        assignments.insert(connected[0].clone(), TemplateType::Full);
    } else {
        // Multiple monitors: FULL on the preferred one, SIMPLE on the others
        let preferred = &cfg.display.preferred_monitor;

        for monitor in connected {
            if monitor == preferred {
                assignments.insert(monitor.clone(), TemplateType::Full);
            } else {
                assignments.insert(monitor.clone(), TemplateType::Simple);
            }
        }
    }

    assignments
}

pub fn launch_waybar_instances(
    cfg: &Config,
    connected: &[String],
    wm: &WindowManager,
    verbose: bool,
) -> Result<()> {
    let config_assignments = determine_config_assignments(cfg, connected);
    let style_path = dirs::home_dir()
        .unwrap()
        .join(".config/waybar/omynix_style.css");

    for (monitor, template_type) in &config_assignments {
        let config_path = get_generated_config_path(wm, monitor, template_type);

        let type_str = match template_type {
            TemplateType::Full => "FULL".green(),
            TemplateType::Simple => "SIMPLE".blue(),
            TemplateType::Custom(name) => name.yellow(),
        };

        if verbose {
            println!("Implement -- launch_waybar_instances()");
        }


        println!("  {} Starting waybar {} in: {}", "→".cyan(), type_str, monitor.cyan());

        Command::new("waybar")
            .arg("-c")
            .arg(&config_path)
            .arg("-s")
            .arg(&style_path)
            .spawn()
            .context("Error launching waybar")?;

        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Ok(())
}
