use crate::pi_config::agent_dir;
use serde_json::{json, Value};
use std::{fs, io::Write, path::Path};

// Same short-lived directory lock used by Pi's proper-lockfile (realpath: false).
pub fn patch(path: &Path, change: impl FnOnce(&mut Value) -> Result<(), String>) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing config directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let lock = path.with_file_name(format!("{}.lock", path.file_name().unwrap().to_string_lossy()));
    fs::create_dir(&lock).map_err(|_| "Configuration is locked by another process. Retry after it finishes.".to_string())?;
    struct Unlock(std::path::PathBuf);
    impl Drop for Unlock { fn drop(&mut self) { let _ = fs::remove_dir(&self.0); } }
    let _unlock = Unlock(lock);
    let mut data = read(path)?;
    if !data.is_object() { return Err("Configuration must be a JSON object; nothing was changed".into()); }
    change(&mut data)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        file.as_file().set_permissions(fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    file.write_all(serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn read(path: &Path) -> Result<Value, String> {
    match fs::read(path) {
        Ok(raw) => serde_json::from_slice(&raw).map_err(|_| format!("Invalid JSON in {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.to_string()),
    }
}
fn provider_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 100 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)) { return Err("Provider ID must use letters, numbers, dashes, dots or underscores".into()); }
    Ok(())
}
#[tauri::command]
pub fn pi_save_enabled_models(models: Vec<String>) -> Result<(), String> {
    if models.len() > 10000 || models.iter().any(|m| m.is_empty() || m.len() > 300 || m.contains(['\n', '\r', ','])) { return Err("Invalid model scope".into()); }
    patch(&agent_dir().join("settings.json"), |v| { v["enabledModels"] = json!(models); Ok(()) })
}
#[tauri::command]
pub fn pi_remove_api_key(provider: String) -> Result<(), String> {
    provider_id(&provider)?;
    patch(&agent_dir().join("auth.json"), |v| {
        if v[&provider]["type"] != "api_key" { return Err("Only API keys can be removed here. Manage OAuth using pi /logout.".into()); }
        v.as_object_mut().unwrap().remove(&provider); Ok(())
    })
}
pub fn save_key(provider: &str, key: &str) -> Result<(), String> {
    provider_id(provider)?;
    if key.trim().is_empty() || key.len() > 16384 || key.contains(['\n', '\r']) { return Err("Invalid API key".into()); }
    patch(&agent_dir().join("auth.json"), |v| {
        if !v[provider].is_null() && v[provider]["type"] != "api_key" { return Err("This provider uses OAuth; log out explicitly before replacing it.".into()); }
        // Pi resolves !commands and $variables in key fields; encode user input as a literal.
        let literal = key.replace('$', "$$");
        let literal = if literal.starts_with('!') { format!("${literal}") } else { literal };
        v[provider] = json!({"type":"api_key", "key":literal}); Ok(())
    })
}
#[tauri::command]
pub fn pi_custom_providers() -> Result<Value, String> {
    let data = read(&agent_dir().join("models.json"))?;
    let mut out = Vec::new();
    if let Some(providers) = data["providers"].as_object() {
        for (id, p) in providers { out.push(json!({"id":id, "baseUrl":p["baseUrl"], "api":p["api"], "models":p["models"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str()).collect::<Vec<_>>()).unwrap_or_default()})); }
    }
    Ok(json!(out))
}
#[tauri::command]
pub fn pi_save_custom_provider(provider: String, base_url: String, api: String, models: Vec<String>) -> Result<(), String> {
    provider_id(&provider)?;
    let url = reqwest::Url::parse(&base_url).map_err(|_| "Invalid endpoint URL")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() { return Err("Endpoint must be HTTP(S), without credentials, query or fragment".into()); }
    if !["openai-completions", "openai-responses", "anthropic-messages", "google-generative-ai"].contains(&api.as_str()) { return Err("Unsupported provider API".into()); }
    if models.is_empty() || models.len() > 1000 || models.iter().any(|id| id.trim().is_empty() || id.len() > 300 || id.contains(['\n','\r'])) { return Err("Supply one or more valid model IDs".into()); }
    patch(&agent_dir().join("models.json"), |v| {
        if v["providers"].is_null() { v["providers"] = json!({}); }
        let providers = v["providers"].as_object_mut().ok_or("Invalid providers object")?;
        let p = providers.entry(provider).or_insert(json!({}));
        if !p.is_object() { return Err("Existing provider configuration is invalid".into()); }
        let old = p["models"].as_array().cloned().unwrap_or_default();
        let models: Vec<_> = models.iter().map(|id| old.iter().find(|m| m["id"] == *id).cloned().unwrap_or(json!({"id":id}))).collect();
        p["baseUrl"] = json!(base_url); p["api"] = json!(api); p["models"] = json!(models); Ok(())
    })
}
#[tauri::command]
pub fn pi_remove_custom_provider(provider: String) -> Result<(), String> {
    provider_id(&provider)?;
    patch(&agent_dir().join("models.json"), |v| { if let Some(p) = v["providers"].as_object_mut() { p.remove(&provider); } Ok(()) })
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn atomic_patch_preserves_fields_and_rejects_bad_json() {
        let d = tempfile::tempdir().unwrap(); let p = d.path().join("settings.json");
        fs::write(&p, r#"{"other":{"keep":true}}"#).unwrap();
        patch(&p, |v| { v["enabledModels"] = json!(["test/model"]); Ok(()) }).unwrap();
        assert_eq!(read(&p).unwrap()["other"]["keep"], true);
        fs::write(&p, "broken").unwrap(); assert!(patch(&p, |_| Ok(())).is_err()); assert_eq!(fs::read_to_string(&p).unwrap(), "broken");
        fs::create_dir(d.path().join("settings.json.lock")).unwrap(); assert!(patch(&p, |_| Ok(())).is_err());
    }
}
