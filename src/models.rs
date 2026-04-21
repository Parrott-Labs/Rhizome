use std::fmt;
use serde::{Deserialize, Serialize};

// ── ProjectStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Lead,
    Pending,
    Active,
    Running,
    Blocked,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lead     => "lead",
            Self::Pending  => "pending",
            Self::Active   => "active",
            Self::Running  => "running",
            Self::Blocked  => "blocked",
            Self::Archived => "archived",
        }
    }

    pub fn color(self) -> ratatui::style::Color {
        crate::theme::status_color(self.as_str())
    }
}

impl fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for ProjectStatus {
    fn default() -> Self { Self::Pending }
}

impl From<&str> for ProjectStatus {
    fn from(s: &str) -> Self {
        match s {
            "lead"     => Self::Lead,
            "active"   => Self::Active,
            "running"  => Self::Running,
            "blocked"  => Self::Blocked,
            "archived" => Self::Archived,
            _          => Self::Pending,
        }
    }
}

// ── BrokenProject ────────────────────────────────────────────────────────────

/// A project TOML file that failed to deserialize (e.g. unknown status string).
/// We keep enough info to display a warning and let the user repair it.
#[derive(Debug, Clone)]
pub struct BrokenProject {
    pub path:       std::path::PathBuf,
    pub name:       String,
    pub id:         String,
    pub client_id:  String,
    pub bad_status: String,
    pub raw_toml:   String,
}

pub const PROJECT_STATUSES: &[ProjectStatus] = &[
    ProjectStatus::Lead,
    ProjectStatus::Pending,
    ProjectStatus::Active,
    ProjectStatus::Running,
    ProjectStatus::Blocked,
    ProjectStatus::Archived,
];

// ── Client ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub primary_contact: Option<String>,
    #[serde(default)]
    pub primary_role: Option<String>,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub default_rate_cents: Option<i64>,
    #[serde(default = "default_active")]
    pub status: String,
    #[serde(default)]
    pub acct_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_currency() -> String { "EUR".into() }
fn default_active()   -> String { "active".into() }

impl Client {
    pub fn health_color(&self) -> ratatui::style::Color {
        crate::theme::status_color(&self.status)
    }
    pub fn location_str(&self) -> String {
        match (&self.city, &self.country) {
            (Some(c), Some(co)) => format!("{} · {}", c, co),
            (Some(c), None)     => c.clone(),
            (None, Some(co))    => co.clone(),
            _                   => String::new(),
        }
    }
    pub fn rate_display(&self) -> String {
        self.default_rate_cents
            .map(|c| format!("€ {} / h", c / 100))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub client_id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub status: ProjectStatus,
    #[serde(default)]
    pub budget_hours: Option<f64>,
    #[serde(default = "default_budget_kind")]
    pub budget_kind: String,  // "total" | "monthly"
    #[serde(default)]
    pub spent_hours: f64,
    #[serde(default)]
    pub rate_cents: Option<i64>,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub repo_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    #[serde(default)]
    pub pulse: Vec<u8>, // 8 values 0–8 for sparkline
    // runtime only (not persisted)
    #[serde(skip)]
    pub client_name: Option<String>,
}

fn default_budget_kind() -> String { "total".into() }

impl Project {
    pub fn status_color(&self) -> ratatui::style::Color {
        self.status.color()
    }
    pub fn spent_pct(&self) -> Option<f64> {
        self.budget_hours
            .filter(|&b| b > 0.0)
            .map(|b| self.spent_hours / b)
    }
    pub fn deadline_display(&self) -> String {
        use chrono::{Local, NaiveDate};
        match &self.deadline {
            None => String::new(),
            Some(d) => {
                let today = Local::now().date_naive();
                if let Ok(due) = NaiveDate::parse_from_str(d, "%Y-%m-%d") {
                    let days = (due - today).num_days();
                    if days < 0 {
                        format!("{} · {}d ago", d, -days)
                    } else {
                        format!("{} · {}d", d, days)
                    }
                } else {
                    d.clone()
                }
            }
        }
    }
    pub fn rate_display(&self) -> String {
        self.rate_cents
            .map(|c| format!("€ {} / h", c / 100))
            .unwrap_or_default()
    }
    pub fn sparkline_str(&self) -> String {
        const SPARKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let data: Vec<u8> = if self.pulse.is_empty() {
            vec![0; 8]
        } else {
            let mut p = self.pulse.clone();
            p.resize(8, 0);
            p
        };
        data.iter().map(|&v| SPARKS[v.min(8) as usize]).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub done_at: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
    pub ord: i64,
}

impl Milestone {
    pub fn is_done(&self) -> bool { self.done_at.is_some() }
    pub fn symbol(&self) -> &str {
        if self.is_done() { "✓" } else { "·" }
    }
    pub fn symbol_color(&self) -> ratatui::style::Color {
        if self.is_done() { crate::theme::GREEN } else { crate::theme::SUBTLE }
    }
    pub fn due_display(&self) -> String {
        self.due_at.as_deref().unwrap_or("").to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub id: String,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    pub kind: String,
    pub message: String,
    pub at: String,
    #[serde(default)]
    pub hours: Option<f64>,
}

impl ActivityEntry {
    pub fn symbol(&self) -> &str {
        match self.kind.as_str() {
            "deploy" | "invoice" => "✓",
            "warn"               => "!",
            "call"               => "→",
            _                    => "·",
        }
    }
    pub fn symbol_color(&self) -> ratatui::style::Color {
        match self.kind.as_str() {
            "deploy" | "invoice" => crate::theme::GREEN,
            "warn"               => crate::theme::AMBER,
            "call"               => crate::theme::BLUE,
            _                    => crate::theme::ACCENT,
        }
    }
    pub fn at_short(&self) -> &str {
        // Return first 16 chars of ISO timestamp: "2026-04-02 11:30"
        if self.at.len() >= 16 { &self.at[..16] } else { &self.at }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactMoment {
    pub id: String,
    pub client_id: String,
    pub kind: String,    // email | phone | meeting
    pub summary: String,
    pub date: String,    // YYYY-MM-DD
    pub created_at: String,
}

impl ContactMoment {
    pub fn kind_symbol(&self) -> &str {
        match self.kind.as_str() {
            "phone"   => "☎",
            "meeting" => "◈",
            _         => "✉",
        }
    }
    pub fn kind_color(&self) -> ratatui::style::Color {
        match self.kind.as_str() {
            "phone"   => crate::theme::GREEN,
            "meeting" => crate::theme::AMBER,
            _         => crate::theme::CYAN,
        }
    }
}

pub const CONTACT_KINDS: &[&str] = &["email", "phone", "meeting"];

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub total_clients: usize,
    pub active_projects: usize,
    pub total_projects: usize,
    pub attention_count: usize,
    pub hours_this_week: f64,
    pub clients_this_quarter: usize,
}
