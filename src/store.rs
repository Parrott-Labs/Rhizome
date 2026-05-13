use std::io;
use std::path::Path;

use crate::models::{
    ActivityEntry, BrokenProject, Client, ContactMoment, Project, ProjectStatus, Stats,
};
use crate::storage::Storage;
use chrono::{Local, Datelike, NaiveDate};

pub struct Store<'a> {
    storage: &'a Storage,
}

impl<'a> Store<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    // ── Clients ──────────────────────────────────────────────────────────────

    pub fn load_clients(&self) -> io::Result<Vec<Client>> {
        let mut clients = load_toml_dir(&self.storage.clients_dir())?;
        clients.sort_by(|a: &Client, b: &Client| a.name.cmp(&b.name));
        Ok(clients)
    }

    pub fn save_client(&self, client: &Client) -> io::Result<()> {
        let path = self
            .storage
            .clients_dir()
            .join(format!("{}.toml", client.id));
        let text = toml::to_string_pretty(client)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        crate::storage::atomic_write(&path, text.as_bytes())
    }

    pub fn delete_client(&self, id: &str) -> io::Result<()> {
        let path = self.storage.clients_dir().join(format!("{}.toml", id));
        std::fs::remove_file(path)
    }

    // ── Projects ─────────────────────────────────────────────────────────────

    pub fn load_projects(&self, clients: &[Client]) -> io::Result<Vec<Project>> {
        let mut projects: Vec<Project> = load_toml_dir(&self.storage.projects_dir())?;
        // Join client names
        for p in &mut projects {
            p.client_name = clients
                .iter()
                .find(|c| c.id == p.client_id)
                .map(|c| c.name.clone());
        }
        projects.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(projects)
    }

    pub fn save_project(&self, project: &Project) -> io::Result<()> {
        let path = self
            .storage
            .projects_dir()
            .join(format!("{}.toml", project.id));
        let text = toml::to_string_pretty(project)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        crate::storage::atomic_write(&path, text.as_bytes())
    }

    pub fn delete_project(&self, id: &str) -> io::Result<()> {
        let path = self.storage.projects_dir().join(format!("{}.toml", id));
        std::fs::remove_file(path)
    }

    // ── Activity ─────────────────────────────────────────────────────────────

    pub fn load_activity(&self) -> io::Result<Vec<ActivityEntry>> {
        let mut entries: Vec<ActivityEntry> = load_toml_dir(&self.storage.activity_dir())?;
        entries.sort_by(|a, b| b.at.cmp(&a.at)); // newest first
        Ok(entries)
    }

    // ── Contacts ─────────────────────────────────────────────────────────────

    pub fn load_contacts(&self) -> io::Result<Vec<ContactMoment>> {
        let mut entries: Vec<ContactMoment> = load_toml_dir(&self.storage.contacts_dir())?;
        entries.sort_by(|a, b| b.date.cmp(&a.date));
        Ok(entries)
    }

    pub fn save_contact(&self, c: &ContactMoment) -> io::Result<()> {
        let path = self.storage.contacts_dir().join(format!("{}.toml", c.id));
        let text =
            toml::to_string_pretty(c).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        crate::storage::atomic_write(&path, text.as_bytes())
    }

    pub fn delete_contact(&self, id: &str) -> io::Result<()> {
        let path = self.storage.contacts_dir().join(format!("{}.toml", id));
        std::fs::remove_file(path)
    }

    pub fn delete_activity(&self, id: &str) -> io::Result<()> {
        let path = self.storage.activity_dir().join(format!("{}.toml", id));
        std::fs::remove_file(path)
    }

    pub fn save_activity(&self, entry: &ActivityEntry) -> io::Result<()> {
        let path = self
            .storage
            .activity_dir()
            .join(format!("{}.toml", entry.id));
        let text = toml::to_string_pretty(entry)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        crate::storage::atomic_write(&path, text.as_bytes())
    }

    // ── Stats ─────────────────────────────────────────────────────────────────

    pub fn compute_stats(clients: &[Client], projects: &[Project], activities: &[ActivityEntry]) -> Stats {
        let total_clients = clients.len();
        let active_projects = projects
            .iter()
            .filter(|p| matches!(p.status, ProjectStatus::Running | ProjectStatus::Active))
            .count();
        let total_projects = projects.len();
        let attention_count = projects
            .iter()
            .filter(|p| {
                p.status == ProjectStatus::Blocked || {
                    p.spent_pct().map(|pct| pct > 0.85).unwrap_or(false)
                }
            })
            .count();

        let today = Local::now().date_naive();
        let hours_this_week: f64 = activities
            .iter()
            .filter(|activity| {
                activity.hours.is_some() && NaiveDate::parse_from_str(&activity.at[..10], "%Y-%m-%d")
                .map(|d| d.iso_week() == today.iso_week() && d.year() == today.year())
                .unwrap_or(false)
            })
            .map(|activity| activity.hours.unwrap_or(0.0))
            .sum();

        let clients_this_quarter = 0usize; // populated once time-tracking lands
        Stats {
            total_clients,
            active_projects,
            total_projects,
            attention_count,
            hours_this_week,
            clients_this_quarter,
        }
    }

    // ── Broken project detection ──────────────────────────────────────────────

    /// Scan the projects directory for TOML files that fail full deserialization.
    /// For each broken file, parse it as a raw TOML value to extract display info.
    pub fn load_broken_projects(&self) -> io::Result<Vec<BrokenProject>> {
        let dir = self.storage.projects_dir();
        let mut out = Vec::new();
        if !dir.exists() {
            return Ok(out);
        }
        for entry in dir.read_dir()? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let text = std::fs::read_to_string(&path)?;
            if toml::from_str::<Project>(&text).is_ok() {
                continue;
            }
            // Failed — try to extract basic info from raw TOML
            if let Ok(raw) = toml::from_str::<toml::Value>(&text) {
                let get = |k: &str| {
                    raw.get(k)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                };
                out.push(BrokenProject {
                    path,
                    name: {
                        let n = get("name");
                        if n.is_empty() { "unknown".into() } else { n }
                    },
                    id: get("id"),
                    client_id: get("client_id"),
                    bad_status: get("status"),
                    raw_toml: text,
                });
            }
        }
        Ok(out)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_toml_dir<T: serde::de::DeserializeOwned>(dir: &Path) -> io::Result<Vec<T>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in dir.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let text = std::fs::read_to_string(&path)?;
            match toml::from_str::<T>(&text) {
                Ok(v) => out.push(v),
                Err(e) => eprintln!("warn: skipping {}: {}", path.display(), e),
            }
        }
    }
    Ok(out)
}
