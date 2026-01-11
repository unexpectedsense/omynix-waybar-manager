use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub template_hash: String,
    pub monitors: Vec<String>,
    pub preferred_monitor: String,
    pub timestamp: i64,
}

pub fn get_cache_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("The home directory could not be retrieved")?;
    Ok(home.join(".local/share/omynix/waybar-manager/waybar_cache.toml"))
}

pub fn load_cache() -> Result<Option<CacheEntry>> {
    let cache_path = get_cache_path()?;
    
    if !cache_path.exists() {
        return Ok(None);
    }
    
    let contents = fs::read_to_string(&cache_path)
        .context("The cache file could not be read")?;
    
    let cache: CacheEntry = toml::from_str(&contents)
        .context("Error parsing cache file")?;
    
    Ok(Some(cache))
}

pub fn save_cache(cache: &CacheEntry) -> Result<()> {
    let cache_path = get_cache_path()?;
    
    // Create directory if it does not exist
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .context("The cache directory could not be created")?;
    }
    
    let toml_string = toml::to_string_pretty(cache)
        .context("Error serializing cache")?;
    
    fs::write(&cache_path, toml_string)
        .context("Error writing cache file")?;
    
    Ok(())
}

pub fn calculate_template_hash(template_content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    template_content.hash(&mut hasher);
    hasher.finish().to_string()
}

pub fn get_current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64 
}

pub fn should_regenerate(
    cache: Option<&CacheEntry>,
    template_hash: &str,
    monitors: &[String],
    preferred_monitor: &str,
    generated_files_exist: bool,
) -> bool {
    // If there is no cache, regenerate
    let Some(cache) = cache else {
        return true;
    };
    
    // If the generated files do not exist, regenerate
    if !generated_files_exist {
        return true;
    }
    
    // If the template hash changed, regenerate
    if cache.template_hash != template_hash {
        return true;
    }
    
    // If you changed your preferred monitor, regenerate
    if cache.preferred_monitor != preferred_monitor {
        return true;
    }
    
    // If the monitor list has changed, regenerate
    let mut cache_monitors = cache.monitors.clone();
    let mut current_monitors = monitors.to_vec();
    cache_monitors.sort();
    current_monitors.sort();
    
    if cache_monitors != current_monitors {
        return true;
    }
    
    // Everything matches up, not regenerating
    false
}

pub fn check_generated_files_exist(
    monitors: &[String],
    wm: &crate::window_manager::WindowManager,
) -> bool {
    use crate::templates::{get_generated_config_path, TemplateType};
    
    // Verify that files exist for at least all monitors
    for monitor in monitors {
        // Verify at least one type (full or simple)
        let full_path = get_generated_config_path(wm, monitor, &TemplateType::Full);
        let simple_path = get_generated_config_path(wm, monitor, &TemplateType::Simple);
        
        if !full_path.exists() && !simple_path.exists() {
            return false;
        }
    }
    
    true
}
