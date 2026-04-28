use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const API_URL: &str = "https://api.github.com/repos/Parrott-Labs/Rhizome/releases/latest";

#[derive(Clone, Deserialize, Serialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

#[derive(Serialize, Deserialize)]
struct Cache {
    release: Release,
}

fn is_newer(local: &str, remote: &str) -> Option<bool> {
    let parse = |s: &str| -> Option<Vec<u32>> {
        s.trim_start_matches('v')
            .split('.')
            .take(3)
            .map(|p| p.parse::<u32>().ok())
            .collect::<Option<Vec<_>>>()
    };
    Some(parse(remote)? > parse(local)?)
}

fn cache_path() -> Option<PathBuf> {
    ProjectDirs::from("", "Parrott-Labs", "rhizome")
        .map(|d: ProjectDirs| d.cache_dir().join("update-check.json"))
}

fn load_cache() -> Option<Release> {
    let contents = fs::read_to_string(cache_path()?).ok()?;
    serde_json::from_str::<Cache>(&contents)
        .ok()
        .map(|c| c.release)
}

fn save_cache(release: &Release) {
    let Some(path) = cache_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(json) = serde_json::to_string(&Cache {
        release: release.clone(),
    }) {
        let _ = fs::write(path, json);
    }
}

fn cache_is_fresh() -> bool {
    let Some(path) = cache_path() else {
        return false;
    };
    fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|age| age.as_secs() < CHECK_INTERVAL_SECS)
        .unwrap_or(false)
}

fn fetch() -> Option<Release> {
    let response = ureq::get(API_URL)
        .set(
            "User-Agent",
            &format!("rhizome/{}", env!("CARGO_PKG_VERSION")),
        )
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(5))
        .call()
        .ok()?;

    if response.status() != 200 {
        return None;
    }

    let body = response.into_string().ok()?;
    serde_json::from_str(&body).ok()
}

/// Spawns a background thread that sends an update message if a newer release exists.
/// The sender fires at most once, then the thread exits.
pub fn spawn_check() -> std::sync::mpsc::Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let release = if cache_is_fresh() {
            load_cache()
        } else {
            match fetch() {
                Some(r) => {
                    save_cache(&r);
                    Some(r)
                }
                None => load_cache(),
            }
        };

        if let Some(r) = release {
            if is_newer(env!("CARGO_PKG_VERSION"), &r.tag_name).unwrap_or(false) {
                let msg = format!("update available → {}", r.tag_name);
                let _ = tx.send(msg);
            }
        }
    });
    return rx;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison() {
        assert_eq!(is_newer("0.1.0", "0.2.0"), Some(true));
        assert_eq!(is_newer("0.2.0", "0.2.0"), Some(false));
        assert_eq!(is_newer("1.0.0", "0.9.9"), Some(false));
        assert_eq!(is_newer("0.9.0", "0.10.0"), Some(true));
        assert_eq!(is_newer("v0.1.0", "v0.2.0"), Some(true));

        // None cases, prevents panic
        assert_eq!(is_newer("not-a-version", "0.2.0"), None);
        assert_eq!(is_newer("0.2.0", "not-a-version"), None);
        assert_eq!(is_newer("", ""), None);
    }
}
