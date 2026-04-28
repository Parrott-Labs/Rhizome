use ratatui::widgets::ListState;

use crate::config::Config;
use crate::models::{ActivityEntry, BrokenProject, Client, ContactMoment, Project, ProjectStatus, Stats, CONTACT_KINDS, PROJECT_STATUSES};
use crate::store::Store;
use crate::storage::Storage;
use crate::utils::{non_empty, parse_hours_from_message, step_list};

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Search(String),
    Command(String),
    ConfirmDelete { kind: String, id: String, name: String, warning: Option<String>, context: Option<String> },
    NewClient(NewClientForm),
    NewProject(NewProjectForm),
    NewContact(NewContactForm),
    ViewContacts,
    AddHours { project_id: String, input: String, error: Option<String> },
    ViewHours { project_id: String },
    EditHours { project_id: String, activity_id: String, date_input: String, hours_input: String, focused: usize, error: Option<String> },
    RepairProject { idx: usize, chosen: ProjectStatus, error: Option<String> },
}

// ── New-client form ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct NewClientForm {
    pub fields: Vec<FormField>,
    pub focused: usize,
    pub error: Option<String>,
    pub edit_id: Option<String>, // Some(id) = editing existing, None = creating new
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    pub label: &'static str,
    pub value: String,
    pub required: bool,
}

impl NewClientForm {
    pub fn new() -> Self {
        Self {
            fields: vec![
                FormField { label: "name",      value: String::new(),  required: true  },
                FormField { label: "domain",    value: String::new(),  required: false },
                FormField { label: "city",      value: String::new(),  required: false },
                FormField { label: "country",   value: String::new(),  required: false },
                FormField { label: "contact",   value: String::new(),  required: false },
                FormField { label: "role",      value: String::new(),  required: false },
                FormField { label: "currency",  value: "EUR".into(),   required: false },
                FormField { label: "rate €/h",  value: String::new(),  required: false },
            ],
            focused: 0,
            error: None,
            edit_id: None,
        }
    }

    pub fn from_client(client: &Client) -> Self {
        let rate_str = client.default_rate_cents
            .map(|c| (c / 100).to_string())
            .unwrap_or_default();
        Self {
            fields: vec![
                FormField { label: "name",     value: client.name.clone(),                                    required: true  },
                FormField { label: "domain",   value: client.domain.clone().unwrap_or_default(),              required: false },
                FormField { label: "city",     value: client.city.clone().unwrap_or_default(),                required: false },
                FormField { label: "country",  value: client.country.clone().unwrap_or_default(),             required: false },
                FormField { label: "contact",  value: client.primary_contact.clone().unwrap_or_default(),     required: false },
                FormField { label: "role",     value: client.primary_role.clone().unwrap_or_default(),        required: false },
                FormField { label: "currency", value: client.currency.clone(),                                required: false },
                FormField { label: "rate €/h", value: rate_str,                                               required: false },
            ],
            focused: 0,
            error: None,
            edit_id: Some(client.id.clone()),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.fields[0].value.trim().is_empty()
    }

    pub fn next_field(&mut self) {
        self.focused = (self.focused + 1) % self.fields.len();
    }

    pub fn prev_field(&mut self) {
        self.focused = self.focused.checked_sub(1).unwrap_or(self.fields.len() - 1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Dashboard,
    Clients,
    Projects,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BootPhase {
    Logo,
    Lines,
    Done,
}

// ── Search results ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SearchResult {
    Client  { idx: usize, name: String, subtitle: String },
    Project { idx: usize, name: String, client: String, status: String },
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub should_quit: bool,
    pub active_view: View,
    pub mode: Mode,
    pub focus: Focus,
    pub config: Config,

    // Data
    pub clients: Vec<Client>,
    pub projects: Vec<Project>,
    pub activity: Vec<ActivityEntry>,
    pub contacts: Vec<ContactMoment>,
    pub stats: Stats,
    pub broken_projects: Vec<BrokenProject>,

    // List selection state
    pub client_list: ListState,
    pub project_list: ListState,
    pub dashboard_list: ListState,
    pub contact_list: ListState,
    pub hours_list: ListState,
    pub search_list: ListState,
    pub search_results: Vec<SearchResult>,

    // Animation ticks
    pub tick_count: u64,

    // Boot sequence
    pub boot_phase: BootPhase,
    pub boot_lines_shown: usize,
    pub boot_tick: u64,       // tick at which boot phase last advanced
    pub boot_skip: bool,
    pub boot_lines: Vec<(String, String)>,

    // Input buffer for Search / Command modes
    pub input: String,

    // g-stroke pending (waiting for gd/gc/gp/gg)
    pub g_pending: bool,

    // Paths
    pub storage: Storage,
}

impl App {
    pub fn new() -> std::io::Result<Self> {
        let storage = Storage::new()?;
        let config = Config::load_or_default(&storage.config_file());
        let store = Store::new(&storage);

        let clients = store.load_clients()?;
        let mut projects = store.load_projects(&clients)?;
        // Sort projects by updated_at desc for dashboard recent list
        projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        let activity = store.load_activity()?;
        let contacts = store.load_contacts()?;
        let stats = Store::compute_stats(&clients, &projects);
        let broken_projects = store.load_broken_projects()?;

        let boot_lines = vec![
            ("ok".into(),   "loading config · ~/.config/rhizome/config.toml".to_string()),
            ("ok".into(),   format!("loading clients · {} record{}", clients.len(), if clients.len() == 1 { "" } else { "s" })),
            ("ok".into(),   format!("loading projects · {} record{}", projects.len(), if projects.len() == 1 { "" } else { "s" })),
            ("ok".into(),   format!("loading contacts · {} record{}", contacts.len(), if contacts.len() == 1 { "" } else { "s" })),
            (if broken_projects.is_empty() { "ok".into() } else { "warn".into() },
                            format!("checking integrity · {} broken", broken_projects.len())),
            ("ok".into(),   "terminal · truecolor · utf-8".into()),
            ("boot".into(), "rhizome ready · press any key".into()),
        ];

        let mut client_list = ListState::default();
        if !clients.is_empty() { client_list.select(Some(0)); }
        let mut project_list = ListState::default();
        if !projects.is_empty() { project_list.select(Some(0)); }
        let mut dashboard_list = ListState::default();
        if !projects.is_empty() { dashboard_list.select(Some(0)); }

        Ok(App {
            should_quit: false,
            active_view: View::Dashboard,
            mode: Mode::Normal,
            focus: Focus::Left,
            config,
            clients,
            projects,
            activity,
            contacts,
            stats,
            broken_projects,
            client_list,
            project_list,
            dashboard_list,
            contact_list:    ListState::default(),
            hours_list:      ListState::default(),
            search_list:     ListState::default(),
            search_results:  Vec::new(),
            tick_count: 0,
            boot_phase: BootPhase::Logo,
            boot_lines_shown: 0,
            boot_tick: 0,
            boot_skip: false,
            boot_lines,
            input: String::new(),
            g_pending: false,
            storage,
        })
    }

    // ── Boot sequence ─────────────────────────────────────────────────────────

    pub fn boot_done(&self) -> bool {
        self.boot_phase == BootPhase::Done
    }

    pub fn skip_boot(&mut self) {
        self.boot_phase = BootPhase::Done;
        self.boot_skip = true;
    }

    // ── Tick ──────────────────────────────────────────────────────────────────

    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Boot sequencing (timing in 120 ms ticks)
        match self.boot_phase {
            BootPhase::Logo => {
                if self.tick_count.saturating_sub(self.boot_tick) >= 2 {
                    self.boot_phase = BootPhase::Lines;
                    self.boot_tick = self.tick_count;
                }
            }
            BootPhase::Lines => {
                if self.tick_count.saturating_sub(self.boot_tick) >= 1 {
                    self.boot_lines_shown = (self.boot_lines_shown + 2).min(self.boot_lines.len());
                    self.boot_tick = self.tick_count;
                    if self.boot_lines_shown >= self.boot_lines.len() {
                        self.boot_phase = BootPhase::Done;
                    }
                }
            }
            BootPhase::Done => {}
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn switch_view(&mut self, view: View) {
        if self.active_view != view {
            self.active_view = view;
            self.focus = Focus::Left;
        }
    }

    pub fn nav_down(&mut self) {
        match self.active_view {
            View::Dashboard => step_list(&mut self.dashboard_list, self.projects.len(), 1),
            View::Clients   => step_list(&mut self.client_list,    self.clients.len(),  1),
            View::Projects  => step_list(&mut self.project_list,   self.projects.len(), 1),
        }
    }

    pub fn nav_up(&mut self) {
        match self.active_view {
            View::Dashboard => step_list(&mut self.dashboard_list, self.projects.len(), -1),
            View::Clients   => step_list(&mut self.client_list,    self.clients.len(),  -1),
            View::Projects  => step_list(&mut self.project_list,   self.projects.len(), -1),
        }
    }

    pub fn nav_first(&mut self) {
        match self.active_view {
            View::Dashboard => self.dashboard_list.select(Some(0)),
            View::Clients   => self.client_list.select(Some(0)),
            View::Projects  => self.project_list.select(Some(0)),
        }
    }

    pub fn nav_last(&mut self) {
        match self.active_view {
            View::Dashboard => {
                let n = self.projects.len();
                if n > 0 { self.dashboard_list.select(Some(n - 1)); }
            }
            View::Clients => {
                let n = self.clients.len();
                if n > 0 { self.client_list.select(Some(n - 1)); }
            }
            View::Projects => {
                let n = self.projects.len();
                if n > 0 { self.project_list.select(Some(n - 1)); }
            }
        }
    }

    pub fn selected_client(&self) -> Option<&Client> {
        self.client_list.selected().and_then(|i| self.clients.get(i))
    }

    pub fn selected_project(&self) -> Option<&Project> {
        self.project_list.selected().and_then(|i| self.projects.get(i))
    }

    pub fn selected_dashboard_project(&self) -> Option<&Project> {
        self.dashboard_list.selected().and_then(|i| self.projects.get(i))
    }

    pub fn client_projects(&self, client_id: &str) -> Vec<&Project> {
        self.projects.iter().filter(|p| p.client_id == client_id).collect()
    }

    pub fn client_activity(&self, client_id: &str) -> Vec<&ActivityEntry> {
        self.activity.iter().filter(|a| {
            a.client_id.as_deref() == Some(client_id)
        }).take(5).collect()
    }

    pub fn client_contacts(&self, client_id: &str) -> Vec<&ContactMoment> {
        self.contacts.iter().filter(|c| c.client_id == client_id).collect()
    }

    pub fn selected_contact(&self) -> Option<&ContactMoment> {
        let cid = self.selected_client()?.id.clone();
        let list: Vec<&ContactMoment> = self.contacts.iter()
            .filter(|c| c.client_id == cid)
            .collect();
        self.contact_list.selected().and_then(|i| list.get(i).copied())
    }

    pub fn nav_contact(&mut self, delta: i64) {
        let count = self.selected_client()
            .map(|c| self.contacts.iter().filter(|ct| ct.client_id == c.id).count())
            .unwrap_or(0);
        if count == 0 { return; }
        let cur = self.contact_list.selected().unwrap_or(0) as i64;
        let next = (cur + delta).clamp(0, count as i64 - 1) as usize;
        self.contact_list.select(Some(next));
    }

    pub fn enter_contact_list(&mut self) {
        let has = self.selected_client()
            .map(|c| self.contacts.iter().any(|ct| ct.client_id == c.id))
            .unwrap_or(false);
        if has {
            self.contact_list.select(Some(0));
        } else {
            self.contact_list.select(None);
        }
    }

    pub fn delete_contact(&mut self, id: &str) -> std::io::Result<()> {
        let store = Store::new(&self.storage);
        store.delete_contact(id)?;
        self.contacts = store.load_contacts()?;
        // Clamp selection
        let count = self.selected_client()
            .map(|c| self.contacts.iter().filter(|ct| ct.client_id == c.id).count())
            .unwrap_or(0);
        if count == 0 {
            self.contact_list.select(None);
        } else {
            let i = self.contact_list.selected().unwrap_or(0).min(count - 1);
            self.contact_list.select(Some(i));
        }
        Ok(())
    }

    /// Days since last contact; None if no contact moments exist.
    pub fn days_since_contact(&self, client_id: &str) -> Option<i64> {
        use chrono::{Local, NaiveDate};
        let today = Local::now().date_naive();
        self.contacts.iter()
            .filter(|c| c.client_id == client_id)
            .filter_map(|c| NaiveDate::parse_from_str(&c.date, "%Y-%m-%d").ok())
            .max()
            .map(|last| (today - last).num_days())
    }

    /// Clients with no contact in the last 7 days (or never contacted and older than 7 days).
    pub fn init_search(&mut self) {
        self.search_results.clear();
        self.search_list.select(None);
    }

    pub fn global_search(&mut self, q: &str) {
        if q.is_empty() {
            self.search_results.clear();
            self.search_list.select(None);
            return;
        }
        let ql = q.to_lowercase();
        let mut results = Vec::new();

        for (idx, c) in self.clients.iter().enumerate() {
            if c.name.to_lowercase().contains(&ql)
                || c.domain.as_deref().unwrap_or("").to_lowercase().contains(&ql)
                || c.city.as_deref().unwrap_or("").to_lowercase().contains(&ql)
                || c.country.as_deref().unwrap_or("").to_lowercase().contains(&ql)
                || c.primary_contact.as_deref().unwrap_or("").to_lowercase().contains(&ql)
            {
                results.push(SearchResult::Client {
                    idx,
                    name:     c.name.clone(),
                    subtitle: c.location_str(),
                });
            }
        }

        for (idx, p) in self.projects.iter().enumerate() {
            if p.name.to_lowercase().contains(&ql)
                || p.client_name.as_deref().unwrap_or("").to_lowercase().contains(&ql)
                || p.summary.as_deref().unwrap_or("").to_lowercase().contains(&ql)
                || p.status.as_str().contains(ql.as_str())
                || p.owner.as_deref().unwrap_or("").to_lowercase().contains(&ql)
            {
                results.push(SearchResult::Project {
                    idx,
                    name:   p.name.clone(),
                    client: p.client_name.clone().unwrap_or_default(),
                    status: p.status.to_string(),
                });
            }
        }

        self.search_results = results;
        if self.search_results.is_empty() {
            self.search_list.select(None);
        } else {
            self.search_list.select(Some(0));
        }
    }

    pub fn nav_search(&mut self, delta: i64) {
        let count = self.search_results.len();
        if count == 0 { self.search_list.select(None); return; }
        let cur = self.search_list.selected().unwrap_or(0) as i64;
        let next = (cur + delta).clamp(0, count as i64 - 1) as usize;
        self.search_list.select(Some(next));
    }

    pub fn commit_global_search(&mut self) {
        let sel = self.search_list.selected().unwrap_or(0);
        if let Some(result) = self.search_results.get(sel).cloned() {
            match result {
                SearchResult::Client { idx, .. } => {
                    self.switch_view(View::Clients);
                    self.client_list.select(Some(idx));
                }
                SearchResult::Project { idx, .. } => {
                    self.switch_view(View::Projects);
                    self.project_list.select(Some(idx));
                    self.dashboard_list.select(Some(idx));
                }
            }
        }
    }

    pub fn stale_clients(&self) -> Vec<(&Client, i64)> {
        use chrono::{Local, NaiveDate};
        let today = Local::now().date_naive();
        self.clients.iter().filter_map(|c| {
            match self.days_since_contact(&c.id) {
                Some(d) if d >= 7 => Some((c, d)),
                None => {
                    // Never contacted — use created_at as baseline
                    let created = NaiveDate::parse_from_str(&c.created_at[..10], "%Y-%m-%d").ok()?;
                    let d = (today - created).num_days();
                    if d >= 7 { Some((c, d)) } else { None }
                }
                _ => None,
            }
        }).collect()
    }

    // ── Mutations ─────────────────────────────────────────────────────────────

    pub fn repair_project(&mut self, idx: usize, new_status: ProjectStatus) -> std::io::Result<()> {
        let broken = self.broken_projects[idx].clone();
        // Rewrite the status line in the raw TOML, leaving everything else intact
        let fixed: String = broken.raw_toml.lines().map(|line| {
            if line.trim_start().starts_with("status") {
                format!("status = \"{}\"", new_status.as_str())
            } else {
                line.to_string()
            }
        }).collect::<Vec<_>>().join("\n");
        crate::storage::atomic_write(&broken.path, fixed.as_bytes())?;
        let store = Store::new(&self.storage);
        self.projects = store.load_projects(&self.clients)?;
        self.broken_projects = store.load_broken_projects()?;
        self.stats = Store::compute_stats(&self.clients, &self.projects);
        Ok(())
    }

    pub fn delete_project(&mut self, id: &str) -> std::io::Result<()> {
        let store = Store::new(&self.storage);
        store.delete_project(id)?;
        // Reload projects and recompute stats
        self.projects = store.load_projects(&self.clients)?;
        self.stats = Store::compute_stats(&self.clients, &self.projects);
        // Clamp selection so it doesn't point past the end
        let new_len = self.projects.len();
        if new_len == 0 {
            self.project_list.select(None);
            self.dashboard_list.select(None);
        } else {
            let clamped = self.project_list.selected()
                .unwrap_or(0)
                .min(new_len - 1);
            self.project_list.select(Some(clamped));
            let clamped2 = self.dashboard_list.selected()
                .unwrap_or(0)
                .min(new_len - 1);
            self.dashboard_list.select(Some(clamped2));
        }
        Ok(())
    }

    pub fn delete_client(&mut self, id: &str) -> std::io::Result<()> {
        let store = Store::new(&self.storage);
        // Archive every project belonging to this client before removing it
        for p in self.projects.iter_mut().filter(|p| p.client_id == id) {
            p.status = ProjectStatus::Archived;
            store.save_project(p)?;
        }
        store.delete_client(id)?;
        // Reload everything
        self.clients  = store.load_clients()?;
        self.projects = store.load_projects(&self.clients)?;
        self.stats    = Store::compute_stats(&self.clients, &self.projects);
        // Clamp list selections
        let cn = self.clients.len();
        if cn == 0 {
            self.client_list.select(None);
        } else {
            let i = self.client_list.selected().unwrap_or(0).min(cn - 1);
            self.client_list.select(Some(i));
        }
        let pn = self.projects.len();
        if pn == 0 {
            self.project_list.select(None);
            self.dashboard_list.select(None);
        } else {
            let i = self.project_list.selected().unwrap_or(0).min(pn - 1);
            self.project_list.select(Some(i));
            let i2 = self.dashboard_list.selected().unwrap_or(0).min(pn - 1);
            self.dashboard_list.select(Some(i2));
        }
        Ok(())
    }

    pub fn save_new_client(&mut self, form: &NewClientForm) -> std::io::Result<()> {
        use uuid::Uuid;
        use chrono::Utc;

        let name = form.fields[0].value.trim().to_string();
        let now  = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let rate_cents = form.fields[7].value.trim()
            .parse::<f64>()
            .ok()
            .map(|r| (r * 100.0) as i64);

        let currency = {
            let c = form.fields[6].value.trim();
            if c.is_empty() { "EUR".to_string() } else { c.to_uppercase() }
        };

        let store = Store::new(&self.storage);

        let saved_id = if let Some(ref id) = form.edit_id {
            // Update existing client — preserve id, acct_code, status, created_at
            let existing = self.clients.iter().find(|c| &c.id == id).cloned();
            let client = Client {
                id:                 id.clone(),
                name,
                domain:             non_empty(&form.fields[1].value),
                city:               non_empty(&form.fields[2].value),
                country:            non_empty(&form.fields[3].value),
                primary_contact:    non_empty(&form.fields[4].value),
                primary_role:       non_empty(&form.fields[5].value),
                currency,
                default_rate_cents: rate_cents,
                status:             existing.as_ref().map(|c| c.status.clone()).unwrap_or_else(|| "active".into()),
                acct_code:          existing.as_ref().and_then(|c| c.acct_code.clone()),
                created_at:         existing.as_ref().map(|c| c.created_at.clone()).unwrap_or_else(|| now.clone()),
                updated_at:         now,
            };
            store.save_client(&client)?;
            client.id
        } else {
            // Create new client
            let next = self.clients.len() + 1;
            let client = Client {
                id:                 Uuid::new_v4().to_string(),
                name,
                domain:             non_empty(&form.fields[1].value),
                city:               non_empty(&form.fields[2].value),
                country:            non_empty(&form.fields[3].value),
                primary_contact:    non_empty(&form.fields[4].value),
                primary_role:       non_empty(&form.fields[5].value),
                currency,
                default_rate_cents: rate_cents,
                status:             "active".into(),
                acct_code:          Some(format!("n-{:03}", next)),
                created_at:         now.clone(),
                updated_at:         now,
            };
            store.save_client(&client)?;
            client.id
        };

        self.clients = store.load_clients()?;
        self.stats   = Store::compute_stats(&self.clients, &self.projects);

        if let Some(idx) = self.clients.iter().position(|c| c.id == saved_id) {
            self.client_list.select(Some(idx));
        }
        Ok(())
    }

    pub fn save_contact_moment(&mut self, form: &NewContactForm) -> std::io::Result<()> {
        use uuid::Uuid;
        use chrono::Utc;

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let id = form.edit_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let created_at = form.edit_id.as_ref()
            .and_then(|eid| self.contacts.iter().find(|c| &c.id == eid))
            .map(|c| c.created_at.clone())
            .unwrap_or_else(|| now.clone());

        let contact = ContactMoment {
            id,
            client_id:  form.client_id.clone(),
            kind:       form.fields[1].value.clone(),
            summary:    form.fields[2].value.trim().to_string(),
            date:       form.fields[0].value.trim().to_string(),
            created_at,
        };

        let store = Store::new(&self.storage);
        store.save_contact(&contact)?;
        self.contacts = store.load_contacts()?;
        Ok(())
    }

    pub fn save_hours(&mut self, project_id: &str, hours: f64) -> std::io::Result<()> {
        use uuid::Uuid;
        use chrono::Utc;

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let store = Store::new(&self.storage);

        let project = self.projects.iter_mut().find(|p| p.id == project_id)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "project not found"))?;
        project.spent_hours += hours;
        project.updated_at = now.clone();
        store.save_project(project)?;

        let project_name = project.name.clone();
        let client_id = project.client_id.clone();
        let entry = crate::models::ActivityEntry {
            id:         Uuid::new_v4().to_string(),
            client_id:  Some(client_id),
            project_id: Some(project_id.to_string()),
            kind:       "hours".into(),
            message:    format!("+{} h · {}", hours, project_name),
            at:         now,
            hours:      Some(hours),
        };
        store.save_activity(&entry)?;

        self.projects = store.load_projects(&self.clients)?;
        self.activity = store.load_activity()?;
        self.stats    = Store::compute_stats(&self.clients, &self.projects);
        Ok(())
    }

    pub fn project_hours(&self, project_id: &str) -> Vec<&crate::models::ActivityEntry> {
        self.activity.iter()
            .filter(|a| a.project_id.as_deref() == Some(project_id) && a.kind == "hours")
            .collect()
    }

    pub fn nav_hours(&mut self, delta: i64, count: usize) {
        if count == 0 { self.hours_list.select(None); return; }
        let cur = self.hours_list.selected().unwrap_or(0) as i64;
        let next = (cur + delta).clamp(0, count as i64 - 1) as usize;
        self.hours_list.select(Some(next));
    }

    pub fn selected_hours_entry(&self, project_id: &str) -> Option<&crate::models::ActivityEntry> {
        let entries = self.project_hours(project_id);
        self.hours_list.selected().and_then(|i| entries.get(i).copied())
    }

    pub fn delete_hours_entry(&mut self, activity_id: &str) -> std::io::Result<()> {
        let store = Store::new(&self.storage);
        if let Some(entry) = self.activity.iter().find(|a| a.id == activity_id).cloned() {
            let hours = entry.hours
                .unwrap_or_else(|| parse_hours_from_message(&entry.message).unwrap_or(0.0));
            if let Some(pid) = &entry.project_id
                && let Some(project) = self.projects.iter_mut().find(|p| p.id == *pid) {
                    project.spent_hours = (project.spent_hours - hours).max(0.0);
                    project.updated_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                    store.save_project(project)?;
                }
            store.delete_activity(activity_id)?;
        }
        self.activity = store.load_activity()?;
        self.projects = store.load_projects(&self.clients)?;
        self.stats    = Store::compute_stats(&self.clients, &self.projects);
        Ok(())
    }

    pub fn edit_hours_entry(&mut self, activity_id: &str, new_hours: f64, new_date: &str) -> std::io::Result<()> {
        let store = Store::new(&self.storage);
        if let Some(entry) = self.activity.iter().find(|a| a.id == activity_id).cloned() {
            let old_hours = entry.hours
                .unwrap_or_else(|| parse_hours_from_message(&entry.message).unwrap_or(0.0));
            let project_name = entry.project_id.as_ref()
                .and_then(|pid| self.projects.iter().find(|p| p.id == *pid))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            // Replace date portion of timestamp, preserve time
            let new_at = if entry.at.len() > 10 {
                format!("{}{}", new_date, &entry.at[10..])
            } else {
                format!("{}T00:00:00Z", new_date)
            };
            let updated = crate::models::ActivityEntry {
                id:         entry.id.clone(),
                client_id:  entry.client_id.clone(),
                project_id: entry.project_id.clone(),
                kind:       "hours".into(),
                message:    format!("+{} h · {}", new_hours, project_name),
                at:         new_at,
                hours:      Some(new_hours),
            };
            store.save_activity(&updated)?;
            if let Some(pid) = &entry.project_id
                && let Some(project) = self.projects.iter_mut().find(|p| p.id == *pid) {
                    project.spent_hours = (project.spent_hours - old_hours + new_hours).max(0.0);
                    project.updated_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                    store.save_project(project)?;
                }
        }
        self.activity = store.load_activity()?;
        self.projects = store.load_projects(&self.clients)?;
        self.stats    = Store::compute_stats(&self.clients, &self.projects);
        Ok(())
    }

    pub fn save_new_project(&mut self, form: &NewProjectForm) -> std::io::Result<()> {
        use uuid::Uuid;
        use chrono::Utc;

        let name        = form.fields[0].value.trim().to_string();
        let client_name = form.fields[1].value.trim().to_lowercase();
        let now         = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        // Resolve client by name (case-insensitive prefix match)
        let client = self.clients.iter().find(|c| {
            c.name.to_lowercase().starts_with(&client_name)
        });
        let client_id = match client {
            Some(c) => c.id.clone(),
            None => return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no client matching '{}'", form.fields[1].value.trim()),
            )),
        };

        let budget_kind  = form.fields[4].value.clone();
        let budget_hours = non_empty(&form.fields[5].value)
            .and_then(|v| v.parse::<f64>().ok());
        let rate_cents = non_empty(&form.fields[6].value)
            .and_then(|v| v.parse::<f64>().ok())
            .map(|r| (r * 100.0) as i64);
        let status = ProjectStatus::from(form.fields[3].value.trim());

        let existing = form.edit_id.as_ref()
            .and_then(|eid| self.projects.iter().find(|p| &p.id == eid))
            .cloned();

        let project = Project {
            id:           form.edit_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
            client_id,
            name,
            summary:      non_empty(&form.fields[2].value),
            status,
            budget_hours,
            budget_kind,
            spent_hours:  existing.as_ref().map(|p| p.spent_hours).unwrap_or(0.0),
            rate_cents,
            deadline:     non_empty(&form.fields[7].value),
            owner:        non_empty(&form.fields[8].value),
            repo_url:     non_empty(&form.fields[9].value),
            created_at:   existing.as_ref().map(|p| p.created_at.clone()).unwrap_or_else(|| now.clone()),
            updated_at:   now,
            milestones:   existing.as_ref().map(|p| p.milestones.clone()).unwrap_or_default(),
            pulse:        existing.as_ref().map(|p| p.pulse.clone()).unwrap_or_default(),
            client_name:  None,
        };

        let store = Store::new(&self.storage);
        store.save_project(&project)?;

        let saved_id = project.id.clone();
        self.projects = store.load_projects(&self.clients)?;
        self.stats    = Store::compute_stats(&self.clients, &self.projects);

        if let Some(idx) = self.projects.iter().position(|p| p.id == saved_id) {
            self.project_list.select(Some(idx));
            self.dashboard_list.select(Some(idx));
        }
        Ok(())
    }
}

// ── New-project form ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct NewProjectForm {
    pub fields: Vec<FormField>,
    pub focused: usize,
    pub error: Option<String>,
    pub edit_id: Option<String>,
}

pub const BUDGET_KINDS: &[&str] = &["total", "monthly"];

impl NewProjectForm {
    // Field indices: 0 name, 1 client, 2 summary, 3 status, 4 budget (kind), 5 budget h, 6 rate €/h, 7 deadline, 8 owner, 9 repo

    pub fn new() -> Self {
        Self {
            fields: vec![
                FormField { label: "name",     value: String::new(),   required: true  },
                FormField { label: "client",   value: String::new(),   required: true  },
                FormField { label: "summary",  value: String::new(),   required: false },
                FormField { label: "status",   value: "active".into(), required: false },
                FormField { label: "budget",   value: "total".into(),  required: false },
                FormField { label: "budget h", value: String::new(),   required: false },
                FormField { label: "rate €/h", value: String::new(),   required: false },
                FormField { label: "deadline", value: String::new(),   required: false },
                FormField { label: "owner",    value: String::new(),   required: false },
                FormField { label: "repo",     value: String::new(),   required: false },
            ],
            focused: 0,
            error: None,
            edit_id: None,
        }
    }

    pub fn from_project(project: &Project) -> Self {
        let rate_str = project.rate_cents
            .map(|c| (c / 100).to_string())
            .unwrap_or_default();
        Self {
            fields: vec![
                FormField { label: "name",     value: project.name.clone(),                              required: true  },
                FormField { label: "client",   value: project.client_name.clone().unwrap_or_default(),   required: true  },
                FormField { label: "summary",  value: project.summary.clone().unwrap_or_default(),       required: false },
                FormField { label: "status",   value: project.status.to_string(),                        required: false },
                FormField { label: "budget",   value: project.budget_kind.clone(),                       required: false },
                FormField { label: "budget h", value: project.budget_hours.map(|h| h.to_string()).unwrap_or_default(), required: false },
                FormField { label: "rate €/h", value: rate_str,                                          required: false },
                FormField { label: "deadline", value: project.deadline.clone().unwrap_or_default(),      required: false },
                FormField { label: "owner",    value: project.owner.clone().unwrap_or_default(),         required: false },
                FormField { label: "repo",     value: project.repo_url.clone().unwrap_or_default(),      required: false },
            ],
            focused: 0,
            error: None,
            edit_id: Some(project.id.clone()),
        }
    }

    pub fn cycle_budget_kind(&mut self) {
        let cur = BUDGET_KINDS.iter().position(|&k| k == self.fields[4].value.as_str()).unwrap_or(0);
        self.fields[4].value = BUDGET_KINDS[(cur + 1) % BUDGET_KINDS.len()].to_string();
    }

    pub fn cycle_status(&mut self, forward: bool) {
        let n = PROJECT_STATUSES.len();
        let cur = PROJECT_STATUSES.iter().position(|s| s.as_str() == self.fields[3].value).unwrap_or(0);
        let next = if forward { (cur + 1) % n } else { (cur + n - 1) % n };
        self.fields[3].value = PROJECT_STATUSES[next].as_str().to_string();
    }

    pub fn is_valid(&self) -> bool {
        !self.fields[0].value.trim().is_empty() && !self.fields[1].value.trim().is_empty()
    }

    pub fn next_field(&mut self) { self.focused = (self.focused + 1) % self.fields.len(); }
    pub fn prev_field(&mut self) { self.focused = self.focused.checked_sub(1).unwrap_or(self.fields.len() - 1); }
}

// ── New-contact form ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct NewContactForm {
    pub fields: Vec<FormField>,
    // 0: date (YYYY-MM-DD)
    // 1: kind (email | phone | meeting — cycles)
    // 2: summary
    pub focused: usize,
    pub error: Option<String>,
    pub client_id: String,
    pub client_name: String,
    pub edit_id: Option<String>,
}

impl NewContactForm {
    pub fn for_client(client: &Client) -> Self {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        Self {
            fields: vec![
                FormField { label: "date",    value: today,           required: true  },
                FormField { label: "type",    value: "email".into(),  required: true  },
                FormField { label: "summary", value: String::new(),   required: true  },
            ],
            focused: 0,
            error: None,
            client_id:   client.id.clone(),
            client_name: client.name.clone(),
            edit_id:     None,
        }
    }

    pub fn from_contact(client: &Client, contact: &ContactMoment) -> Self {
        Self {
            fields: vec![
                FormField { label: "date",    value: contact.date.clone(),    required: true },
                FormField { label: "type",    value: contact.kind.clone(),    required: true },
                FormField { label: "summary", value: contact.summary.clone(), required: true },
            ],
            focused: 0,
            error: None,
            client_id:   client.id.clone(),
            client_name: client.name.clone(),
            edit_id:     Some(contact.id.clone()),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.fields[0].value.trim().is_empty() && !self.fields[2].value.trim().is_empty()
    }

    pub fn cycle_kind_forward(&mut self) {
        let cur = CONTACT_KINDS.iter().position(|&k| k == self.fields[1].value.as_str()).unwrap_or(0);
        self.fields[1].value = CONTACT_KINDS[(cur + 1) % CONTACT_KINDS.len()].to_string();
    }

    pub fn cycle_kind_backward(&mut self) {
        let cur = CONTACT_KINDS.iter().position(|&k| k == self.fields[1].value.as_str()).unwrap_or(0);
        self.fields[1].value = CONTACT_KINDS[(cur + CONTACT_KINDS.len() - 1) % CONTACT_KINDS.len()].to_string();
    }

    pub fn next_field(&mut self) { self.focused = (self.focused + 1) % self.fields.len(); }
    pub fn prev_field(&mut self) { self.focused = self.focused.checked_sub(1).unwrap_or(self.fields.len() - 1); }
}

