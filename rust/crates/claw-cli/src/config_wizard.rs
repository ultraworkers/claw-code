use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use inquire::{Confirm, Select, Text};

const PROFILES_FILENAME: &str = "profiles.json";

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct ProviderProfile {
    base_url: String,
    api_key: String,
    model: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
struct ProfilesData {
    active: Option<String>,
    profiles: BTreeMap<String, ProviderProfile>,
}

fn profiles_path() -> io::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    Ok(cwd.join(PROFILES_FILENAME))
}

fn load_profiles() -> ProfilesData {
    let path = match profiles_path() {
        Ok(p) => p,
        Err(_) => return ProfilesData::default(),
    };
    let content = match fs::read_to_string(&path) {
        Ok(c) if !c.trim().is_empty() => c,
        _ => return auto_import_from_dotenv(),
    };
    match serde_json::from_str(&content) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Warning: failed to parse {PROFILES_FILENAME} ({e}), starting fresh");
            ProfilesData::default()
        }
    }
}

fn auto_import_from_dotenv() -> ProfilesData {
    let model = std::env::var("ANTHROPIC_MODEL").unwrap_or_default();
    if !model.is_empty() {
        let mut data = ProfilesData::default();
        let profile = ProviderProfile {
            base_url: std::env::var("ANTHROPIC_BASE_URL").unwrap_or_default(),
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            model,
        };
        data.profiles.insert("default".to_string(), profile);
        data.active = Some("default".to_string());
        if profiles_path().is_ok() {
            let _ = save_profiles(&data);
        }
        data
    } else {
        ProfilesData::default()
    }
}

fn save_profiles(data: &ProfilesData) -> io::Result<()> {
    let path = profiles_path()?;
    let content = serde_json::to_string_pretty(data)?;
    fs::write(&path, content)?;
    Ok(())
}

fn write_dotenv(profile: &ProviderProfile) -> io::Result<()> {
    let cwd = std::env::current_dir()?;
    let env_path = cwd.join(".env");

    let existing = fs::read_to_string(&env_path).unwrap_or_default();
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|l| {
            !l.starts_with("ANTHROPIC_BASE_URL=")
                && !l.starts_with("ANTHROPIC_API_KEY=")
                && !l.starts_with("ANTHROPIC_MODEL=")
                && !l.starts_with("CLAW_WORKSPACE_POLICY=")
        })
        .map(|l| l.to_string())
        .collect();

    lines.push(format!("ANTHROPIC_BASE_URL={}", profile.base_url));
    lines.push(format!("ANTHROPIC_API_KEY={}", profile.api_key));
    lines.push(format!("ANTHROPIC_MODEL={}", profile.model));
    lines.push("CLAW_WORKSPACE_POLICY=allow".to_string());
    lines.push(String::new());

    fs::write(&env_path, lines.join("\n"))?;
    std::env::set_var("ANTHROPIC_BASE_URL", &profile.base_url);
    std::env::set_var("ANTHROPIC_API_KEY", &profile.api_key);
    std::env::set_var("ANTHROPIC_MODEL", &profile.model);
    std::env::set_var("CLAW_WORKSPACE_POLICY", "allow");
    Ok(())
}

fn clear_dotenv() -> io::Result<()> {
    let cwd = std::env::current_dir()?;
    let env_path = cwd.join(".env");

    let existing = fs::read_to_string(&env_path).unwrap_or_default();
    let lines: Vec<String> = existing
        .lines()
        .filter(|l| {
            !l.starts_with("ANTHROPIC_BASE_URL=")
                && !l.starts_with("ANTHROPIC_API_KEY=")
                && !l.starts_with("ANTHROPIC_MODEL=")
                && !l.starts_with("CLAW_WORKSPACE_POLICY=")
        })
        .map(|l| l.to_string())
        .collect();

    fs::write(&env_path, lines.join("\n"))?;
    std::env::set_var("ANTHROPIC_BASE_URL", "");
    std::env::set_var("ANTHROPIC_API_KEY", "");
    std::env::set_var("ANTHROPIC_MODEL", "");
    std::env::set_var("CLAW_WORKSPACE_POLICY", "");
    Ok(())
}

fn activate_profile(name: &str, data: &ProfilesData) -> io::Result<()> {
    let Some(profile) = data.profiles.get(name) else {
        return Ok(());
    };
    write_dotenv(profile)?;
    Ok(())
}

fn active_label(data: &ProfilesData) -> String {
    data.active
        .as_deref()
        .map(|n| {
            data.profiles
                .get(n)
                .map(|p| format!("{n} ({})", p.model))
                .unwrap_or_else(|| format!("{n} (missing)"))
        })
        .unwrap_or_else(|| "(none)".to_string())
}

fn profile_names(data: &ProfilesData) -> Vec<String> {
    data.profiles.keys().cloned().collect()
}

fn prompt_edit_profile(existing: Option<&ProviderProfile>) -> Option<ProviderProfile> {
    let default_base = existing.map(|p| p.base_url.as_str()).unwrap_or("http://127.0.0.1:1234");
    let default_key = existing.map(|p| p.api_key.as_str()).unwrap_or("sk-your-key");
    let default_model = existing.map(|p| p.model.as_str()).unwrap_or("");

    let base_url = Text::new("Base URL:")
        .with_default(default_base)
        .prompt()
        .ok()?;

    let api_key = Text::new("API Key:")
        .with_initial_value(default_key)
        .prompt()
        .ok()?;

    let model = Text::new("Model:")
        .with_default(default_model)
        .prompt()
        .ok()?;

    Some(ProviderProfile { base_url, api_key, model })
}

fn prompt_select<'a>(message: &str, options: Vec<&'a str>) -> Option<&'a str> {
    Select::new(message, options).prompt().ok()
}

fn prompt_confirm(message: &str, default: bool) -> Option<bool> {
    Confirm::new(message).with_default(default).prompt().ok()
}

fn prompt_text(message: &str) -> Option<String> {
    Text::new(message)
        .with_validator(|val: &str| {
            if val.trim().is_empty() {
                Err(Box::from("Name cannot be empty"))
            } else if val.contains(' ') {
                Err(Box::from("Name cannot contain spaces"))
            } else {
                Ok(inquire::validator::Validation::Valid)
            }
        })
        .prompt()
        .ok()
}

pub fn profile_models() -> Vec<(String, String)> {
    let data = load_profiles();
    let mut result = Vec::new();
    for (name, profile) in &data.profiles {
        result.push((format!("{} — {}", name, profile.model), profile.model.clone()));
    }
    result
}

pub fn run_wizard() -> io::Result<()> {
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture);

    let mut data = load_profiles();

    loop {
        let status = active_label(&data);
        let choice = match prompt_select(
            &format!("[ Config Wizard ]  Active: {status}"),
            vec![
                "Switch active provider",
                "Add new provider",
                "Edit a provider",
                "Remove a provider",
                "View all profiles",
                "Exit",
            ],
        ) {
            Some(c) => c,
            None => break,
        };

        match choice {
            "Switch active provider" => {
                let names: Vec<String> = profile_names(&data);
                if names.is_empty() {
                    continue;
                }
                let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                let chosen = match prompt_select("Select active profile:", refs) {
                    Some(c) => c.to_string(),
                    None => continue,
                };
                data.active = Some(chosen.clone());
                save_profiles(&data)?;
                activate_profile(&chosen, &data)?;
            }
            "Add new provider" => {
                let name = match prompt_text("Profile name:") {
                    Some(n) => n.trim().to_string(),
                    None => continue,
                };
                if name.is_empty() || data.profiles.contains_key(&name) {
                    continue;
                }
                let profile = match prompt_edit_profile(None) {
                    Some(p) => p,
                    None => continue,
                };
                data.profiles.insert(name.clone(), profile);

                if prompt_confirm("Activate this profile now?", true) == Some(true) {
                    data.active = Some(name.clone());
                    save_profiles(&data)?;
                    activate_profile(&name, &data)?;
                } else {
                    save_profiles(&data)?;
                }
            }
            "Edit a provider" => {
                let names: Vec<String> = profile_names(&data);
                if names.is_empty() {
                    continue;
                }
                let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                let chosen = match prompt_select("Select profile to edit:", refs) {
                    Some(c) => c.to_string(),
                    None => continue,
                };
                let existing = data.profiles.get(&chosen).cloned();
                let updated = match prompt_edit_profile(existing.as_ref()) {
                    Some(p) => p,
                    None => continue,
                };
                data.profiles.insert(chosen.clone(), updated);
                if data.active.as_deref() == Some(&chosen) {
                    activate_profile(&chosen, &data)?;
                }
                save_profiles(&data)?;
            }
            "Remove a provider" => {
                let names: Vec<String> = profile_names(&data);
                if names.is_empty() {
                    continue;
                }
                let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                let chosen = match prompt_select("Select profile to remove:", refs) {
                    Some(c) => c.to_string(),
                    None => continue,
                };
                if prompt_confirm(&format!("Remove '{chosen}'?"), false) == Some(true) {
                    data.profiles.remove(&chosen);
                    if data.active.as_deref() == Some(&chosen) {
                        data.active = None;
                        clear_dotenv()?;
                    }
                    save_profiles(&data)?;
                }
            }
            "View all profiles" => {
                let active = data.active.as_deref();
                let mut lines: Vec<String> = Vec::new();
                for (name, profile) in &data.profiles {
                    let marker = if Some(name.as_str()) == active { "▸" } else { " " };
                    lines.push(format!("{} {} — {}  (model: {})", marker, name, profile.base_url, profile.model));
                }
                if lines.is_empty() {
                    continue;
                }
                let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
                let _ = prompt_select("Profiles (Enter to go back)", refs);
            }
            "Exit" => break,
            _ => break,
        }
    }

    Ok(())
}
