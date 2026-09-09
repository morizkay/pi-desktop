use crate::pi_config::{agent_dir, pi_binary};
use serde::Serialize;
use serde_json::Value;
use std::{fs, io::Read, path::PathBuf, time::Duration};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    source: String,
    installed: Option<String>,
    latest: Option<String>,
    available: bool,
    note: String,
}
fn npm_name(source: &str) -> Option<(&str, bool)> {
    let spec = source.strip_prefix("npm:")?;
    let split = spec
        .char_indices()
        .skip(1)
        .find(|(_, c)| *c == '@')
        .map(|(i, _)| i);
    let name = &spec[..split.unwrap_or(spec.len())];
    let parts: Vec<_> = name.split('/').collect();
    if (name.starts_with('@') && parts.len() != 2)
        || (!name.starts_with('@') && parts.len() != 1)
        || parts
            .iter()
            .any(|p| p.is_empty() || *p == "." || *p == "..")
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@/._-".contains(&b))
    {
        return None;
    }
    Some((name, split.is_some()))
}
fn newer(installed: &str, latest: &str) -> Result<bool, String> {
    let current =
        semver::Version::parse(installed).map_err(|_| "Unrecognized installed version")?;
    let next = semver::Version::parse(latest).map_err(|_| "Unrecognized registry version")?;
    Ok(next > current)
}
fn json_url(url: &str) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|_| "Update check unavailable (network or registry error)")?;
    let mut bytes = Vec::new();
    response
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("Registry response too large".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid version response".into())
}
fn installed_harness() -> Option<(String, PathBuf)> {
    let binary = PathBuf::from(pi_binary());
    let resolved = if binary.components().count() > 1 {
        binary.canonicalize().ok()?
    } else {
        std::env::split_paths(&std::env::var_os("PATH")?)
            .find_map(|p| p.join(&binary).canonicalize().ok())?
    };
    for parent in resolved.ancestors().take(8) {
        let Ok(raw) = fs::read(parent.join("package.json")) else {
            continue;
        };
        let Ok(v) = serde_json::from_slice::<Value>(&raw) else {
            continue;
        };
        if v["name"] == "@earendil-works/pi-coding-agent"
            || v["name"] == "@mariozechner/pi-coding-agent"
        {
            return Some((v["version"].as_str()?.into(), resolved));
        }
    }

    // Volta (and other shims) resolve `pi` to a package binary that is not
    // itself below package.json. Ask the installed CLI for its version instead.
    let output = std::process::Command::new(&binary)
        .arg("--version")
        .output()
        .ok()?;
    let version = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .find_map(|token| semver::Version::parse(token.trim_start_matches('v')).ok())?;
    Some((version.to_string(), resolved))
}
fn check(source: Option<String>) -> Update {
    let mut result = Update {
        source: source.clone().unwrap_or("Pi harness".into()),
        installed: None,
        latest: None,
        available: false,
        note: String::new(),
    };
    let attempt = (|| -> Result<(), String> {
        let (url, pinned) = if let Some(source) = &source {
            let (name, pinned) = npm_name(source).ok_or("Local/git/non-npm source: version comparison not supported; review manually with pi CLI.")?;
            let raw = fs::read(
                agent_dir()
                    .join("npm/node_modules")
                    .join(name)
                    .join("package.json"),
            )
            .map_err(|_| "Installed package manifest not found")?;
            let v: Value =
                serde_json::from_slice(&raw).map_err(|_| "Invalid installed package manifest")?;
            result.installed = v["version"].as_str().map(String::from);
            if pinned {
                result.note =
                    "Pinned npm spec: Pi skips updates. Change the version explicitly to upgrade."
                        .into();
                return Ok(());
            }
            (
                format!(
                    "https://registry.npmjs.org/{}/latest",
                    name.replace('/', "%2f")
                ),
                pinned,
            )
        } else {
            let (version, location) = installed_harness().ok_or(
                "Cannot identify the installed Pi package. Check pi --version in your terminal.",
            )?;
            result.installed = Some(version);
            result.note = if location.to_string_lossy().contains("/Cellar/") {
                "Homebrew installation: use brew upgrade pi-coding-agent in your terminal."
            } else {
                "Use pi update in your terminal to upgrade through Pi's installation manager."
            }
            .into();
            ("https://pi.dev/api/latest-version".into(), false)
        };
        let data = json_url(&url)?;
        let latest = data["version"]
            .as_str()
            .or(data["latestVersion"].as_str())
            .or(data.as_str())
            .ok_or("Version missing from registry response")?;
        result.latest = Some(latest.into());
        result.available = !pinned
            && newer(
                result
                    .installed
                    .as_deref()
                    .ok_or("Installed version unknown")?,
                latest,
            )?;
        if source.is_some() {
            result.note = if result.available {
                "New release verified on the public npm registry."
            } else {
                "Installed version is at least as new as public npm latest."
            }
            .into();
        }
        Ok(())
    })();
    if let Err(error) = attempt {
        result.note = error;
    }
    result
}
#[tauri::command]
pub async fn pi_check_update(source: Option<String>) -> Result<Update, String> {
    if source.as_ref().is_some_and(|s| s.len() > 1000) {
        return Err("Invalid source".into());
    }
    tauri::async_runtime::spawn_blocking(move || check(source))
        .await
        .map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares_versions_not_strings() {
        assert!(newer("1.9.0", "1.10.0").unwrap());
        assert!(!newer("2.0.0", "1.10.0").unwrap());
        assert!(!newer("2.0.0", "2.0.0-beta.1").unwrap());
        assert!(newer("unknown", "2.0.0").is_err());
    }
    #[test]
    fn parses_scopes_pins_and_rejects_paths() {
        assert_eq!(npm_name("npm:@org/pkg"), Some(("@org/pkg", false)));
        assert_eq!(npm_name("npm:@org/pkg@^2"), Some(("@org/pkg", true)));
        assert!(npm_name("npm:../secret").is_none());
        assert!(npm_name("git:host/org/repo").is_none());
    }
}
