use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiSettingsView {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_thinking_level: Option<String>,
    pub packages: Vec<String>,
    pub enabled_models: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiAuthProviderView {
    pub id: String,
    pub auth_type: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiCliResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn agent_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PI_CODING_AGENT_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".pi/agent");
    }
    PathBuf::from(".pi/agent")
}

fn settings_path() -> PathBuf {
    agent_dir().join("settings.json")
}

fn auth_path() -> PathBuf {
    agent_dir().join("auth.json")
}

pub fn pi_binary() -> String {
    std::env::var("PI_DESKTOP_PI_BIN").unwrap_or_else(|_| "pi".to_string())
}

fn run_pi(args: &[&str], cwd: Option<&str>) -> PiCliResult {
    let mut cmd = Command::new(pi_binary());
    cmd.args(args);
    if let Some(c) = cwd {
        cmd.current_dir(c);
    }
    match cmd.output() {
        Ok(out) => PiCliResult {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(e) => PiCliResult {
            success: false,
            stdout: String::new(),
            stderr: e.to_string(),
        },
    }
}

pub fn read_settings() -> Result<PiSettingsView, String> {
    let path = settings_path();
    if !path.is_file() {
        return Ok(PiSettingsView {
            default_provider: None,
            default_model: None,
            default_thinking_level: None,
            packages: vec![],
            enabled_models: vec![],
        });
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let packages = v
        .get("packages")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| package_entry_to_string(item))
                .collect()
        })
        .unwrap_or_default();
    Ok(PiSettingsView {
        default_provider: v
            .get("defaultProvider")
            .and_then(|x| x.as_str())
            .map(String::from),
        default_model: v
            .get("defaultModel")
            .and_then(|x| x.as_str())
            .map(String::from),
        default_thinking_level: v
            .get("defaultThinkingLevel")
            .and_then(|x| x.as_str())
            .map(String::from),
        packages,
        enabled_models: v["enabledModels"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
    })
}

fn package_entry_to_string(item: &Value) -> Option<String> {
    if let Some(s) = item.as_str() {
        return Some(s.to_string());
    }
    item.get("source")
        .and_then(|s| s.as_str())
        .map(String::from)
}

pub fn patch_settings(
    default_provider: Option<String>,
    default_model: Option<String>,
    default_thinking_level: Option<String>,
) -> Result<PiSettingsView, String> {
    let path = settings_path();
    crate::preferences::patch(&path, |v| {
    if let Some(obj) = v.as_object_mut() {
        if let Some(p) = default_provider {
            if p.is_empty() {
                obj.remove("defaultProvider");
            } else {
                obj.insert("defaultProvider".to_string(), json!(p));
            }
        }
        if let Some(m) = default_model {
            if m.is_empty() {
                obj.remove("defaultModel");
            } else {
                obj.insert("defaultModel".to_string(), json!(m));
            }
        }
        if let Some(t) = default_thinking_level {
            if t.is_empty() {
                obj.remove("defaultThinkingLevel");
            } else {
                obj.insert("defaultThinkingLevel".to_string(), json!(t));
            }
        }
    }
    Ok(())
    })?;
    read_settings()
}

pub fn list_auth_providers() -> Result<Vec<PiAuthProviderView>, String> {
    let path = auth_path();
    if !path.is_file() {
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let Some(obj) = v.as_object() else {
        return Ok(vec![]);
    };
    let mut out = Vec::new();
    for (id, entry) in obj {
        let auth_type = entry
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown")
            .to_string();
        out.push(PiAuthProviderView {
            id: id.clone(),
            auth_type,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn set_api_key(provider: &str, key: &str) -> Result<(), String> {
    crate::preferences::save_key(provider, key)
}

pub fn pi_install(source: &str) -> PiCliResult {
    run_pi(&["install", source, "--no-approve"], None)
}

pub fn pi_remove(source: &str) -> PiCliResult {
    run_pi(&["remove", source, "--no-approve"], None)
}

pub fn pi_update_package(source: &str) -> PiCliResult {
    run_pi(&["update", source, "--no-approve"], None)
}

pub fn pi_update_all_extensions() -> PiCliResult {
    run_pi(&["update", "--extensions", "--no-approve"], None)
}

pub fn pi_list_packages() -> PiCliResult {
    run_pi(&["list"], None)
}

pub fn models_store_path() -> PathBuf {
    agent_dir().join("models-store.json")
}

pub fn list_provider_ids_from_store() -> Result<Vec<String>, String> {
    let path = models_store_path();
    if !path.is_file() {
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let Some(obj) = v.as_object() else {
        return Ok(vec![]);
    };
    let mut ids: Vec<String> = obj.keys().cloned().collect();
    ids.sort();
    Ok(ids)
}

pub fn list_models_for_provider(provider: &str) -> Result<Vec<Value>, String> {
    let path = models_store_path();
    if !path.is_file() {
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let models = v
        .get(provider)
        .and_then(|p| p.get("models"))
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(models)
}

pub fn open_login_hint(provider: &str) -> String {
    format!(
        "Run `pi` in a terminal and use /login for provider `{provider}`, or set an API key in Settings."
    )
}

pub fn resolve_models_json_path() -> Option<PathBuf> {
    let p = agent_dir().join("models.json");
    if p.is_file() {
        Some(p)
    } else {
        None
    }
}

pub fn agent_dir_exists() -> bool {
    agent_dir().is_dir()
}

pub fn ensure_agent_dir() -> Result<(), String> {
    std::fs::create_dir_all(agent_dir()).map_err(|e| e.to_string())
}

pub fn normalize_package_source(source: &str) -> String {
    let s = source.trim();
    if s.is_empty() {
        return String::new();
    }
    if s.starts_with("npm:") || s.starts_with("git:") || s.contains('/') || s.contains(':') {
        return s.to_string();
    }
    format!("npm:{s}")
}

pub fn is_path_like(source: &str) -> bool {
    Path::new(source).exists() || source.starts_with("./") || source.starts_with("../")
}
