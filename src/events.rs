use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Focus, Mode, NewClientForm, NewContactForm, NewProjectForm, View};
use crate::models::{ProjectStatus, PROJECT_STATUSES};
use crate::utils::parse_hours_from_message;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    // Any key skips boot
    if !app.boot_done() {
        app.skip_boot();
        return;
    }

    match app.mode.clone() {
        Mode::Normal                            => handle_normal(key, app),
        Mode::Search(_)                         => handle_search(key, app),
        Mode::Command(_)                        => handle_command(key, app),
        Mode::ConfirmDelete { kind, id, context, .. } => handle_confirm_delete(key, app, &kind, &id, context.as_deref()),
        Mode::NewClient(form)                   => handle_new_client(key, app, form),
        Mode::NewProject(form)                  => handle_new_project(key, app, form),
        Mode::NewContact(form)                  => handle_new_contact(key, app, form),
        Mode::ViewContacts                      => handle_view_contacts(key, app),
        Mode::AddHours { project_id, input, error } => handle_add_hours(key, app, project_id, input, error),
        Mode::ViewHours { project_id }          => handle_view_hours(key, app, project_id),
        Mode::EditHours { project_id, activity_id, date_input, hours_input, focused, error } => handle_edit_hours(key, app, project_id, activity_id, date_input, hours_input, focused, error),
        Mode::RepairProject { idx, chosen, .. } => handle_repair_project(key, app, idx, chosen),
    }
}

fn handle_normal(key: KeyEvent, app: &mut App) {
    // g-stroke second key
    if app.g_pending {
        app.g_pending = false;
        match key.code {
            KeyCode::Char('d') => { app.switch_view(View::Dashboard); return; }
            KeyCode::Char('c') => { app.switch_view(View::Clients);   return; }
            KeyCode::Char('p') => { app.switch_view(View::Projects);  return; }
            KeyCode::Char('g') => { app.nav_first();                  return; }
            _ => {} // fall through to normal handling
        }
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => { app.should_quit = true; }

        // Tab navigation
        KeyCode::Char('1') => app.switch_view(View::Dashboard),
        KeyCode::Char('2') => app.switch_view(View::Clients),
        KeyCode::Char('3') => app.switch_view(View::Projects),

        // g-stroke first key
        KeyCode::Char('g') => { app.g_pending = true; }

        // List navigation
        KeyCode::Char('j') | KeyCode::Down => app.nav_down(),
        KeyCode::Char('k') | KeyCode::Up   => app.nav_up(),
        KeyCode::Char('G')                  => app.nav_last(),

        // Delete (list pane focused)
        KeyCode::Char('d') if app.focus == Focus::Left => {
            match app.active_view {
                View::Projects => {
                    if let Some(p) = app.selected_project() {
                        app.mode = Mode::ConfirmDelete {
                            kind:    "project".into(),
                            id:      p.id.clone(),
                            name:    p.name.clone(),
                            warning: None,
                            context: None,
                        };
                    }
                }
                View::Clients => {
                    if let Some(c) = app.selected_client() {
                        let n = app.client_projects(&c.id.clone())
                            .iter()
                            .filter(|p| p.status != ProjectStatus::Archived)
                            .count();
                        let warning = if n > 0 {
                            Some(format!("{} project{} will be archived", n, if n == 1 { "" } else { "s" }))
                        } else {
                            None
                        };
                        app.mode = Mode::ConfirmDelete {
                            kind:    "client".into(),
                            id:      c.id.clone(),
                            name:    c.name.clone(),
                            warning,
                            context: None,
                        };
                    }
                }
                View::Dashboard => {}
            }
        }

        // New client / new project
        KeyCode::Char('n') if app.active_view == View::Clients => {
            app.mode  = Mode::NewClient(NewClientForm::new());
            app.focus = Focus::Right;
        }
        KeyCode::Char('n') if app.active_view == View::Projects => {
            app.mode  = Mode::NewProject(NewProjectForm::new());
            app.focus = Focus::Right;
        }

        // Open contact list for selected client
        KeyCode::Char('c') if app.active_view == View::Clients => {
            if app.selected_client().is_some() {
                app.enter_contact_list();
                app.mode  = Mode::ViewContacts;
                app.focus = Focus::Right;
            }
        }

        // Edit selected client / project
        KeyCode::Char('e') if app.active_view == View::Clients => {
            if let Some(client) = app.selected_client() {
                let form = NewClientForm::from_client(client);
                app.mode  = Mode::NewClient(form);
                app.focus = Focus::Right;
            }
        }
        KeyCode::Char('e') if app.active_view == View::Projects => {
            if let Some(project) = app.selected_project() {
                let form = NewProjectForm::from_project(project);
                app.mode  = Mode::NewProject(form);
                app.focus = Focus::Right;
            }
        }

        // Open hours log for selected project
        KeyCode::Char('l') if app.active_view == View::Projects => {
            if let Some(project) = app.selected_project() {
                let project_id = project.id.clone();
                let count = app.project_hours(&project_id).len();
                app.hours_list.select(if count > 0 { Some(0) } else { None });
                app.mode  = Mode::ViewHours { project_id };
                app.focus = Focus::Right;
            }
        }

        // Repair broken project (dashboard only)
        KeyCode::Char('r') if app.active_view == View::Dashboard && !app.broken_projects.is_empty() => {
            app.mode = Mode::RepairProject {
                idx:    0,
                chosen: ProjectStatus::Active,
                error:  None,
            };
        }

        // Focus switch
        KeyCode::Char('h') | KeyCode::Left  => { app.focus = Focus::Left; }

        // Mode entry
        KeyCode::Char('/') => {
            app.mode = Mode::Search(String::new());
            app.input.clear();
            app.init_search();
        }
        KeyCode::Char(':') => {
            app.mode = Mode::Command(String::new());
            app.input.clear();
        }

        KeyCode::Esc => {
            app.g_pending = false;
        }

        _ => {}
    }
}

fn handle_search(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.clear();
            app.search_results.clear();
            app.search_list.select(None);
        }
        KeyCode::Enter => {
            app.commit_global_search();
            app.mode = Mode::Normal;
            app.input.clear();
            app.search_results.clear();
        }
        KeyCode::Char('j') | KeyCode::Down => { app.nav_search(1); }
        KeyCode::Char('k') | KeyCode::Up   => { app.nav_search(-1); }
        KeyCode::Char(c) => {
            app.input.push(c);
            app.mode = Mode::Search(app.input.clone());
            let q = app.input.clone();
            app.global_search(&q);
        }
        KeyCode::Backspace => {
            app.input.pop();
            app.mode = Mode::Search(app.input.clone());
            let q = app.input.clone();
            app.global_search(&q);
        }
        _ => {}
    }
}

fn handle_new_client(key: KeyEvent, app: &mut App, mut form: NewClientForm) {
    match key.code {
        // Cancel
        KeyCode::Esc => {
            app.mode  = Mode::Normal;
            app.focus = Focus::Left;
        }
        // Save
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if form.is_valid() {
                let _ = app.save_new_client(&form);
                app.mode  = Mode::Normal;
                app.focus = Focus::Left;
            } else {
                form.error = Some("name is required".into());
                app.mode = Mode::NewClient(form);
            }
        }
        // Navigate fields
        KeyCode::Tab | KeyCode::Down => { form.error = None; form.next_field(); app.mode = Mode::NewClient(form); }
        KeyCode::BackTab | KeyCode::Up => { form.error = None; form.prev_field(); app.mode = Mode::NewClient(form); }
        // Enter moves to next field (saves on last)
        KeyCode::Enter => {
            if form.focused == form.fields.len() - 1 {
                if form.is_valid() {
                    let _ = app.save_new_client(&form);
                    app.mode  = Mode::Normal;
                    app.focus = Focus::Left;
                } else {
                    form.focused = 0;
                    form.error   = Some("name is required".into());
                    app.mode = Mode::NewClient(form);
                }
            } else {
                form.next_field();
                app.mode = Mode::NewClient(form);
            }
        }
        // Text editing
        KeyCode::Char(c) => {
            form.error = None;
            form.fields[form.focused].value.push(c);
            app.mode = Mode::NewClient(form);
        }
        KeyCode::Backspace => {
            form.fields[form.focused].value.pop();
            app.mode = Mode::NewClient(form);
        }
        _ => {}
    }
}

fn handle_confirm_delete(key: KeyEvent, app: &mut App, kind: &str, id: &str, context: Option<&str>) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let id = id.to_string();
            match kind {
                "client"      => { app.mode = Mode::Normal; let _ = app.delete_client(&id); }
                "contact"     => { let _ = app.delete_contact(&id); app.mode = Mode::ViewContacts; }
                "hours_entry" => {
                    let project_id = context.unwrap_or("").to_string();
                    let _ = app.delete_hours_entry(&id);
                    let count = app.project_hours(&project_id).len();
                    let sel = app.hours_list.selected().unwrap_or(0);
                    if count == 0 {
                        app.hours_list.select(None);
                    } else {
                        app.hours_list.select(Some(sel.min(count - 1)));
                    }
                    app.mode = Mode::ViewHours { project_id };
                }
                _             => { app.mode = Mode::Normal; let _ = app.delete_project(&id); }
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = match kind {
                "contact"     => Mode::ViewContacts,
                "hours_entry" => Mode::ViewHours { project_id: context.unwrap_or("").to_string() },
                _             => Mode::Normal,
            };
        }
        _ => {}
    }
}

fn handle_view_contacts(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            app.mode  = Mode::Normal;
            app.focus = Focus::Left;
        }
        KeyCode::Char('j') | KeyCode::Down  => app.nav_contact(1),
        KeyCode::Char('k') | KeyCode::Up    => app.nav_contact(-1),
        KeyCode::Char('n') => {
            if let Some(client) = app.selected_client() {
                let form = NewContactForm::for_client(client);
                app.mode = Mode::NewContact(form);
            }
        }
        KeyCode::Char('e') => {
            // Clone to avoid borrow conflict
            let data = app.selected_contact()
                .zip(app.selected_client())
                .map(|(ct, cl)| (cl.clone(), ct.clone()));
            if let Some((client, contact)) = data {
                let form = NewContactForm::from_contact(&client, &contact);
                app.mode = Mode::NewContact(form);
            }
        }
        KeyCode::Char('d') => {
            let data = app.selected_contact()
                .map(|ct| (ct.id.clone(), ct.date.clone()));
            if let Some((id, date)) = data {
                app.mode = Mode::ConfirmDelete {
                    kind:    "contact".into(),
                    id,
                    name:    date,
                    warning: None,
                    context: None,
                };
            }
        }
        _ => {}
    }
}

fn handle_new_contact(key: KeyEvent, app: &mut App, mut form: NewContactForm) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::ViewContacts;
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if form.is_valid() {
                match app.save_contact_moment(&form) {
                    Ok(_)  => { app.mode = Mode::ViewContacts; }
                    Err(e) => { form.error = Some(e.to_string()); app.mode = Mode::NewContact(form); }
                }
            } else {
                form.error = Some("date and summary are required".into());
                app.mode = Mode::NewContact(form);
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            form.error = None;
            form.next_field();
            app.mode = Mode::NewContact(form);
        }
        KeyCode::BackTab | KeyCode::Up => {
            form.error = None;
            form.prev_field();
            app.mode = Mode::NewContact(form);
        }
        KeyCode::Enter => {
            if form.focused == form.fields.len() - 1 {
                if form.is_valid() {
                    match app.save_contact_moment(&form) {
                        Ok(_)  => { app.mode = Mode::ViewContacts; }
                        Err(e) => { form.error = Some(e.to_string()); app.mode = Mode::NewContact(form); }
                    }
                } else {
                    form.focused = 0;
                    form.error   = Some("date and summary are required".into());
                    app.mode = Mode::NewContact(form);
                }
            } else {
                form.next_field();
                app.mode = Mode::NewContact(form);
            }
        }
        // Kind field (index 1) cycles with left/right/space
        KeyCode::Left if form.focused == 1 => {
            form.cycle_kind_backward();
            app.mode = Mode::NewContact(form);
        }
        KeyCode::Right | KeyCode::Char(' ') if form.focused == 1 => {
            form.cycle_kind_forward();
            app.mode = Mode::NewContact(form);
        }
        // Text editing for other fields
        KeyCode::Char(c) if form.focused != 1 => {
            form.error = None;
            form.fields[form.focused].value.push(c);
            app.mode = Mode::NewContact(form);
        }
        KeyCode::Backspace if form.focused != 1 => {
            form.fields[form.focused].value.pop();
            app.mode = Mode::NewContact(form);
        }
        _ => {}
    }
}

fn handle_new_project(key: KeyEvent, app: &mut App, mut form: NewProjectForm) {
    match key.code {
        KeyCode::Esc => {
            app.mode  = Mode::Normal;
            app.focus = Focus::Left;
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if form.is_valid() {
                match app.save_new_project(&form) {
                    Ok(_)    => { app.mode = Mode::Normal; app.focus = Focus::Left; }
                    Err(e)   => { form.error = Some(e.to_string()); app.mode = Mode::NewProject(form); }
                }
            } else {
                form.error = Some("name and client are required".into());
                app.mode = Mode::NewProject(form);
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            form.error = None;
            // Auto-complete client field before advancing
            if form.focused == 1 {
                let typed = form.fields[1].value.trim().to_lowercase();
                if !typed.is_empty() {
                    if let Some(c) = app.clients.iter().find(|c| c.name.to_lowercase().starts_with(&typed)) {
                        form.fields[1].value = c.name.clone();
                    }
                }
            }
            form.next_field();
            app.mode = Mode::NewProject(form);
        }
        KeyCode::BackTab | KeyCode::Up => { form.error = None; form.prev_field(); app.mode = Mode::NewProject(form); }
        KeyCode::Enter => {
            if form.focused == form.fields.len() - 1 {
                if form.is_valid() {
                    match app.save_new_project(&form) {
                        Ok(_)  => { app.mode = Mode::Normal; app.focus = Focus::Left; }
                        Err(e) => { form.error = Some(e.to_string()); app.mode = Mode::NewProject(form); }
                    }
                } else {
                    form.focused = 0;
                    form.error   = Some("name and client are required".into());
                    app.mode = Mode::NewProject(form);
                }
            } else {
                form.next_field();
                app.mode = Mode::NewProject(form);
            }
        }
        KeyCode::Left if form.focused == 3 => {
            form.cycle_status(false);
            app.mode = Mode::NewProject(form);
        }
        KeyCode::Right | KeyCode::Char(' ') if form.focused == 3 => {
            form.cycle_status(true);
            app.mode = Mode::NewProject(form);
        }
        KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') if form.focused == 4 => {
            form.cycle_budget_kind();
            app.mode = Mode::NewProject(form);
        }
        KeyCode::Char(c) if form.focused != 3 && form.focused != 4 => {
            form.error = None;
            form.fields[form.focused].value.push(c);
            app.mode = Mode::NewProject(form);
        }
        KeyCode::Backspace if form.focused != 3 && form.focused != 4 => {
            form.fields[form.focused].value.pop();
            app.mode = Mode::NewProject(form);
        }
        _ => {}
    }
}

fn handle_add_hours(key: KeyEvent, app: &mut App, project_id: String, mut input: String, _error: Option<String>) {
    let save = key.code == KeyCode::Enter
        || (key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL));

    if save {
        match input.trim().parse::<f64>() {
            Ok(h) if h > 0.0 => {
                match app.save_hours(&project_id, h) {
                    Ok(_)  => { app.mode = Mode::ViewHours { project_id }; app.focus = Focus::Right; }
                    Err(e) => {
                        app.mode = Mode::AddHours {
                            project_id,
                            input,
                            error: Some(e.to_string()),
                        };
                    }
                }
            }
            _ => {
                app.mode = Mode::AddHours {
                    project_id,
                    input,
                    error: Some("enter a number > 0 (e.g. 2 or 1.5)".into()),
                };
            }
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::ViewHours { project_id };
            app.focus = Focus::Right;
        }
        KeyCode::Char(c) if c.is_ascii_digit() || (c == '.' && !input.contains('.')) => {
            input.push(c);
            app.mode = Mode::AddHours { project_id, input, error: None };
        }
        KeyCode::Backspace => {
            input.pop();
            app.mode = Mode::AddHours { project_id, input, error: None };
        }
        _ => {}
    }
}

fn handle_view_hours(key: KeyEvent, app: &mut App, project_id: String) {
    let count = app.project_hours(&project_id).len();
    match key.code {
        KeyCode::Esc => {
            app.mode  = Mode::Normal;
            app.focus = Focus::Left;
        }
        KeyCode::Char('j') | KeyCode::Down  => app.nav_hours(1,  count),
        KeyCode::Char('k') | KeyCode::Up    => app.nav_hours(-1, count),
        KeyCode::Char('a') => {
            app.mode = Mode::AddHours {
                project_id,
                input: String::new(),
                error: None,
            };
        }
        KeyCode::Char('e') => {
            let data = app.selected_hours_entry(&project_id)
                .map(|e| {
                    let h = e.hours
                        .unwrap_or_else(|| parse_hours_from_message(&e.message).unwrap_or(0.0));
                    let date = e.at[..e.at.len().min(10)].to_string();
                    (e.id.clone(), h, date)
                });
            if let Some((activity_id, h, date)) = data {
                app.mode = Mode::EditHours {
                    project_id,
                    activity_id,
                    date_input:  date,
                    hours_input: h.to_string(),
                    focused:     0,
                    error:       None,
                };
            }
        }
        KeyCode::Char('d') => {
            let data = app.selected_hours_entry(&project_id)
                .map(|e| {
                    let label = e.at[..10].to_string();
                    let h = e.hours
                        .unwrap_or_else(|| parse_hours_from_message(&e.message).unwrap_or(0.0));
                    (e.id.clone(), label, h)
                });
            if let Some((activity_id, label, h)) = data {
                app.mode = Mode::ConfirmDelete {
                    kind:    "hours_entry".into(),
                    id:      activity_id,
                    name:    format!("{} ({}h)", label, h),
                    warning: None,
                    context: Some(project_id),
                };
            }
        }
        _ => {}
    }
}

fn handle_edit_hours(
    key: KeyEvent,
    app: &mut App,
    project_id: String,
    activity_id: String,
    mut date_input: String,
    mut hours_input: String,
    focused: usize,
    _error: Option<String>,
) {
    let save = key.code == KeyCode::Enter && focused == 1
        || (key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL));

    if save {
        match hours_input.trim().parse::<f64>() {
            Ok(h) if h > 0.0 => {
                match app.edit_hours_entry(&activity_id, h, date_input.trim()) {
                    Ok(_)  => { app.mode = Mode::ViewHours { project_id }; }
                    Err(e) => {
                        app.mode = Mode::EditHours { project_id, activity_id, date_input, hours_input, focused, error: Some(e.to_string()) };
                    }
                }
            }
            _ => {
                app.mode = Mode::EditHours {
                    project_id, activity_id, date_input, hours_input, focused,
                    error: Some("hours must be > 0 (e.g. 2 or 1.5)".into()),
                };
            }
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::ViewHours { project_id };
        }
        // Tab / Enter on date field advances to hours field
        KeyCode::Tab | KeyCode::Down | KeyCode::Enter if focused == 0 => {
            app.mode = Mode::EditHours { project_id, activity_id, date_input, hours_input, focused: 1, error: None };
        }
        KeyCode::BackTab | KeyCode::Up if focused == 1 => {
            app.mode = Mode::EditHours { project_id, activity_id, date_input, hours_input, focused: 0, error: None };
        }
        KeyCode::Char(c) if focused == 0 && (c.is_ascii_digit() || c == '-') => {
            date_input.push(c);
            app.mode = Mode::EditHours { project_id, activity_id, date_input, hours_input, focused, error: None };
        }
        KeyCode::Char(c) if focused == 1 && (c.is_ascii_digit() || (c == '.' && !hours_input.contains('.'))) => {
            hours_input.push(c);
            app.mode = Mode::EditHours { project_id, activity_id, date_input, hours_input, focused, error: None };
        }
        KeyCode::Backspace if focused == 0 => {
            date_input.pop();
            app.mode = Mode::EditHours { project_id, activity_id, date_input, hours_input, focused, error: None };
        }
        KeyCode::Backspace if focused == 1 => {
            hours_input.pop();
            app.mode = Mode::EditHours { project_id, activity_id, date_input, hours_input, focused, error: None };
        }
        _ => {}
    }
}

fn handle_repair_project(key: KeyEvent, app: &mut App, idx: usize, chosen: ProjectStatus) {
    let save = key.code == KeyCode::Enter
        || (key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL));

    if save {
        match app.repair_project(idx, chosen) {
            Ok(_)  => { app.mode = Mode::Normal; }
            Err(e) => {
                app.mode = Mode::RepairProject { idx, chosen, error: Some(e.to_string()) };
            }
        }
        return;
    }

    let n = PROJECT_STATUSES.len();
    let cur = PROJECT_STATUSES.iter().position(|s| *s == chosen).unwrap_or(2);

    match key.code {
        KeyCode::Esc => { app.mode = Mode::Normal; }
        KeyCode::Left => {
            let next = PROJECT_STATUSES[(cur + n - 1) % n];
            app.mode = Mode::RepairProject { idx, chosen: next, error: None };
        }
        KeyCode::Right | KeyCode::Char(' ') => {
            let next = PROJECT_STATUSES[(cur + 1) % n];
            app.mode = Mode::RepairProject { idx, chosen: next, error: None };
        }
        _ => {}
    }
}

fn handle_command(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.clear();
        }
        KeyCode::Enter => {
            let cmd = app.input.trim().to_lowercase();
            app.mode = Mode::Normal;
            app.input.clear();
            match cmd.as_str() {
                "q" | "quit" => { app.should_quit = true; }
                _ => {} // stub: other commands TBD
            }
        }
        KeyCode::Char(c) => {
            app.input.push(c);
            app.mode = Mode::Command(app.input.clone());
        }
        KeyCode::Backspace => {
            app.input.pop();
            app.mode = Mode::Command(app.input.clone());
        }
        _ => {}
    }
}
