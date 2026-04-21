use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Focus, Mode, SearchResult, View};
use crate::models::ProjectStatus;
use crate::theme::*;

mod boot;
mod dashboard;
mod clients;
mod projects;
mod widgets;

pub use widgets::*;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.size();

    // Minimum size guard
    if area.width < 80 || area.height < 24 {
        let msg = Paragraph::new(format!(
            "terminal too small ({}×{}; min 80×24)",
            area.width, area.height
        ))
        .style(fg(RED));
        frame.render_widget(msg, area);
        return;
    }

    // Global layout: topbar(2) + crumbs(1) + content(fill) + cmdbar(1)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_topbar(frame, app, outer[0]);
    render_crumbs(frame, app, outer[1]);
    render_content(frame, app, outer[2]);
    render_cmdbar(frame, app, outer[3]);

    if app.boot_done() {
        if let Mode::Search(_) = &app.mode {
            render_search_popup(frame, app, area);
        }
        if let Mode::RepairProject { .. } = &app.mode {
            render_repair_popup(frame, app, area);
        }
    }
}

// ── Topbar ────────────────────────────────────────────────────────────────────

fn render_topbar(frame: &mut Frame, app: &App, area: Rect) {
    // Row 0: wordmark + tabs
    let row0 = Rect { y: area.y, height: 1, ..area };
    // Row 1: divider line
    let row1 = Rect { y: area.y + 1, height: 1, ..area };

    let left = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(30)])
        .split(row0);

    // Wordmark + tabs (left side)
    let tab = |v: View, label: &str, key: &str| -> Vec<Span<'static>> {
        let active = app.active_view == v;
        vec![
            Span::styled(format!("[{}]", key), fg(BRAND)),
            Span::styled(
                format!("{} ", label),
                if active { bold_fg(ACCENT) } else { fg(SUBTLE) },
            ),
            Span::styled("· ".to_string(), fg(MUTED)),
        ]
    };

    let mut title_spans: Vec<Span> = vec![
        Span::styled("≋ ", bold_fg(BRAND)),
        Span::styled("RHIZOME ", bold_fg(ACCENT)),
        Span::styled("v0.1.0  ", fg(SUBTLE)),
    ];
    title_spans.extend(tab(View::Dashboard, "dashboard", "1"));
    title_spans.extend(tab(View::Clients,   "clients",   "2"));
    title_spans.extend(tab(View::Projects,  "projects",  "3"));

    frame.render_widget(
        Paragraph::new(Line::from(title_spans)).style(Style::default().bg(SURFACE)),
        left[0],
    );

    // Right: heartbeat + clock + db status
    let hb_color = if app.config.animations {
        heartbeat_color(app.tick_count)
    } else {
        BRAND
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("● ".to_string(), fg(hb_color)),
            Span::styled("db ok".to_string(), fg(SUBTLE)),
        ])),
        left[1],
    );

    // Divider row
    let div: String = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(div, fg(BORDER))),
        row1,
    );
}

// ── Crumbs ────────────────────────────────────────────────────────────────────

fn render_crumbs(frame: &mut Frame, app: &App, area: Rect) {
    let section = match app.active_view {
        View::Dashboard => "dashboard",
        View::Clients   => "clients",
        View::Projects  => "projects",
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(40)])
        .split(area);

    // Breadcrumb
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("home", fg(SUBTLE)),
            Span::styled(" / ", fg(MUTED)),
            Span::styled(section.to_string(), fg(TEXT)),
        ])),
        cols[0],
    );

    // Key hints
    let hints = Line::from(vec![
        Span::styled("j k", Style::default().fg(ACCENT).bg(BORDER)),
        Span::styled(" move  ", fg(SUBTLE)),
        Span::styled("/", Style::default().fg(ACCENT).bg(BORDER)),
        Span::styled(" search  ", fg(SUBTLE)),
        Span::styled(":", Style::default().fg(ACCENT).bg(BORDER)),
        Span::styled(" cmd  ", fg(SUBTLE)),
        Span::styled("q", Style::default().fg(ACCENT).bg(BORDER)),
        Span::styled(" quit", fg(SUBTLE)),
    ]);
    frame.render_widget(
        Paragraph::new(hints),
        cols[1],
    );
}

// ── Content ───────────────────────────────────────────────────────────────────

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    // Fill background
    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    if !app.boot_done() {
        boot::render_boot(frame, app, area);
        return;
    }

    match app.active_view {
        View::Dashboard => dashboard::render_dashboard(frame, app, area),
        View::Clients   => clients::render_clients(frame, app, area),
        View::Projects  => projects::render_projects(frame, app, area),
    }
}

// ── Cmdbar ────────────────────────────────────────────────────────────────────

/// Build a key-hint row for Normal mode based on the current view and focus.
fn normal_hints(app: &App) -> Line<'static> {
    // Each entry: (key label, action label)
    let hints: &[(&str, &str)] = match (app.active_view, app.focus) {
        (View::Dashboard, _) => &[
            ("j k",   "move"),
            ("1 2 3", "nav"),
            ("g g",   "top"),
            ("G",     "bottom"),
            ("/",     "search"),
            (":",     "cmd"),
            ("q",     "quit"),
        ],
        (View::Clients, Focus::Left) => &[
            ("j k",  "move"),
            ("n",    "new"),
            ("e",    "edit"),
            ("c",    "contact"),
            ("d",    "delete"),
            ("g g",  "top"),
            ("G",    "bottom"),
            ("q",    "quit"),
        ],
        (View::Clients, Focus::Right) => &[
            ("n",    "new"),
            ("e",    "edit"),
            ("d",    "delete"),
            ("/",    "search"),
            ("q",    "quit"),
        ],
        (View::Projects, Focus::Left) => &[
            ("j k",  "move"),
            ("n",    "new"),
            ("e",    "edit"),
            ("l",    "log"),
            ("d",    "delete"),
            ("g g",  "top"),
            ("G",    "bottom"),
            ("/",    "search"),
            ("q",    "quit"),
        ],
        (View::Projects, Focus::Right) => &[
            ("/",    "search"),
            ("q",    "quit"),
        ],
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (key, label)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", fg(SUBTLE)));
        }
        spans.push(Span::styled(key.to_string(),   Style::default().fg(ACCENT).bg(BORDER)));
        spans.push(Span::styled(format!(" {label}"), fg(SUBTLE)));
    }
    Line::from(spans)
}

// ── Global search popup ───────────────────────────────────────────────────────

fn render_search_popup(frame: &mut Frame, app: &App, area: Rect) {
    let q = match &app.mode {
        Mode::Search(q) => q.clone(),
        _ => return,
    };

    const MAX_VISIBLE: usize = 14;
    let n_results = app.search_results.len().min(MAX_VISIBLE);

    // height = border(2) + query(1) + divider(1) + results + divider(1) + hints(1)
    let popup_h = (n_results as u16 + 6).max(8).min(area.height.saturating_sub(4));
    let popup_w = 74u16.min(area.width.saturating_sub(4));

    let x = area.x + area.width.saturating_sub(popup_w) / 2;
    // Pin popup just below topbar (2) + crumbs (1)
    let y = (area.y + 3).min(area.height.saturating_sub(popup_h));
    let popup_area = Rect { x, y, width: popup_w, height: popup_h };

    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(fg(BRAND))
            .title(Span::styled(" search ", bold_fg(ACCENT))),
        popup_area,
    );

    let inner = Rect {
        x:      popup_area.x + 1,
        y:      popup_area.y + 1,
        width:  popup_area.width.saturating_sub(2),
        height: popup_area.height.saturating_sub(2),
    };

    if inner.height < 4 { return; }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // query input
            Constraint::Length(1), // top divider
            Constraint::Fill(1),   // results list
            Constraint::Length(1), // bottom divider
            Constraint::Length(1), // key hints
        ])
        .split(inner);

    // Query input line
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("/", bold_fg(BRAND)),
            Span::styled(q.clone(), fg(TEXT)),
            Span::styled("█", fg(ACCENT)),
        ])),
        rows[0],
    );

    let div: String = "─".repeat(inner.width as usize);
    frame.render_widget(Paragraph::new(Span::styled(div.clone(), fg(BORDER))), rows[1]);

    // Results
    if app.search_results.is_empty() {
        let placeholder = if q.is_empty() {
            "  type to search across clients and projects"
        } else {
            "  no results"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(placeholder, fg(MUTED))),
            rows[2],
        );
    } else {
        let sel = app.search_list.selected().unwrap_or(0);
        let items: Vec<ListItem> = app.search_results.iter().take(MAX_VISIBLE).enumerate().map(|(i, r)| {
            let selected = i == sel;
            let bg       = if selected { SELECTED_BG } else { BG };
            let edge     = if selected { Span::styled("│", fg(BRAND)) } else { Span::raw(" ") };

            let line = match r {
                SearchResult::Client { name, subtitle, .. } => Line::from(vec![
                    edge,
                    Span::raw(" "),
                    Span::styled("client ", Style::default().fg(CYAN)),
                    Span::raw("  "),
                    Span::styled(
                        widgets::col(name, 22),
                        if selected { bold_fg(ACCENT) } else { fg(TEXT) },
                    ),
                    Span::raw("  "),
                    Span::styled(subtitle.clone(), fg(SUBTLE)),
                ]),
                SearchResult::Project { name, client, status, .. } => Line::from(vec![
                    edge,
                    Span::raw(" "),
                    Span::styled("project", Style::default().fg(GREEN)),
                    Span::raw("  "),
                    Span::styled(
                        widgets::col(name, 22),
                        if selected { bold_fg(ACCENT) } else { fg(TEXT) },
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{} · {}", client, status),
                        fg(SUBTLE),
                    ),
                ]),
            };
            ListItem::new(line).style(Style::default().bg(bg))
        }).collect();

        let mut list_state = app.search_list.clone();
        let list = List::new(items)
            .highlight_style(Style::default().bg(SELECTED_BG))
            .highlight_symbol("");
        frame.render_stateful_widget(list, rows[2], &mut list_state);
    }

    frame.render_widget(Paragraph::new(Span::styled(div, fg(BORDER))), rows[3]);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("j k",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" move  ", fg(SUBTLE)),
            Span::styled("Enter",  Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" jump  ", fg(SUBTLE)),
            Span::styled("Esc",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel", fg(SUBTLE)),
        ])),
        rows[4],
    );
}

fn render_repair_popup(frame: &mut Frame, app: &App, area: Rect) {
    let (idx, chosen, error) = match &app.mode {
        Mode::RepairProject { idx, chosen, error } => (*idx, *chosen, error.clone()),
        _ => return,
    };

    let broken = match app.broken_projects.get(idx) {
        Some(b) => b,
        None => return,
    };

    let has_error = error.is_some();
    // height: border(2) + name(1) + bad_status(1) + spacer(1) + spinner(1) + spacer(1) + hints(1) + optional error(1)
    let popup_h: u16 = if has_error { 10 } else { 9 };
    let popup_w: u16 = 60u16.min(area.width.saturating_sub(4));

    let x = area.x + area.width.saturating_sub(popup_w) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect { x, y, width: popup_w, height: popup_h };

    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(fg(RED))
            .title(Span::styled(" repair project ", bold_fg(AMBER))),
        popup_area,
    );

    let inner = Rect {
        x:      popup_area.x + 1,
        y:      popup_area.y + 1,
        width:  popup_area.width.saturating_sub(2),
        height: popup_area.height.saturating_sub(2),
    };

    // Project name
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("project  ", fg(SUBTLE)),
            Span::styled(broken.name.clone(), bold_fg(ACCENT)),
        ])),
        Rect { y: inner.y, height: 1, ..inner },
    );

    // Bad status line
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("status   ", fg(SUBTLE)),
            Span::styled(format!("\"{}\"", broken.bad_status), fg(RED)),
            Span::styled("  not recognised", fg(MUTED)),
        ])),
        Rect { y: inner.y + 1, height: 1, ..inner },
    );

    // Spacer line (blank)
    // Spinner
    let spinner_display = format!("◀  {}  ▶", chosen.as_str());
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("replace  ", fg(SUBTLE)),
            Span::styled(spinner_display, bold_fg(GREEN)),
        ])),
        Rect { y: inner.y + 3, height: 1, ..inner },
    );

    // Key hints
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("← →",  Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cycle  ", fg(SUBTLE)),
            Span::styled("Enter", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save  ", fg(SUBTLE)),
            Span::styled("Esc",   Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel", fg(SUBTLE)),
        ])),
        Rect { y: inner.y + 5, height: 1, ..inner },
    );

    // Error line (if any)
    if let Some(err) = error {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("✗ {}", err), fg(RED)),
            ])),
            Rect { y: inner.y + 6, height: 1, ..inner },
        );
    }
}

fn render_cmdbar(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(16),
            Constraint::Fill(1),
            Constraint::Length(32),
        ])
        .split(area);

    // Mode indicator
    let (mode_str, mode_color) = match &app.mode {
        Mode::Normal                 => ("── NORMAL ──",     BRAND),
        Mode::Search(_)              => ("── SEARCH ──",     AMBER),
        Mode::Command(_)             => ("── COMMAND ──",    AMBER),
        Mode::ConfirmDelete { .. }   => ("── DELETE ──",     RED),
        Mode::NewClient(f) if f.edit_id.is_some() => ("── EDIT CLIENT ──", CYAN),
        Mode::NewClient(_)                        => ("── NEW CLIENT ──",  GREEN),
        Mode::NewProject(f) if f.edit_id.is_some() => ("── EDIT PROJECT ──", CYAN),
        Mode::NewProject(_)                         => ("── NEW PROJECT ──",  GREEN),
        Mode::NewContact(f) if f.edit_id.is_some() => ("── EDIT CONTACT ──", CYAN),
        Mode::NewContact(_)                         => ("── NEW CONTACT ──",  CYAN),
        Mode::ViewContacts                          => ("── CONTACTS ──",     BRAND),
        Mode::AddHours { .. }                       => ("── ADD HOURS ──",    GREEN),
        Mode::ViewHours { .. }                      => ("── HOURS LOG ──",    BRAND),
        Mode::EditHours { .. }                      => ("── EDIT HOURS ──",   CYAN),
        Mode::RepairProject { .. }                  => ("── REPAIR ──",        RED),
    };
    frame.render_widget(
        Paragraph::new(Span::styled(mode_str.to_string(), bold_fg(mode_color)))
            .style(Style::default().bg(CMDBAR_BG)),
        cols[0],
    );

    // Key hints or input buffer
    let middle = match &app.mode {
        Mode::Normal => normal_hints(app),
        Mode::Search(_) => Line::from(vec![
            Span::styled("j k",   Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" move  ", fg(SUBTLE)),
            Span::styled("Enter", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" jump  ", fg(SUBTLE)),
            Span::styled("Esc",   Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel", fg(SUBTLE)),
        ]),
        Mode::Command(c) => Line::from(vec![
            Span::styled(":".to_string(), fg(ACCENT)),
            Span::styled(c.clone(),       fg(TEXT)),
            Span::styled("█",             fg(ACCENT)),
        ]),
        Mode::ViewContacts => Line::from(vec![
            Span::styled("j k",  Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" move  ",  fg(SUBTLE)),
            Span::styled("n",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" new  ",   fg(SUBTLE)),
            Span::styled("e",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" edit  ",  fg(SUBTLE)),
            Span::styled("d",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" delete  ", fg(SUBTLE)),
            Span::styled("Esc",  Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" back",    fg(SUBTLE)),
        ]),
        Mode::AddHours { .. } => Line::from(vec![
            Span::styled("Enter", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save hours  ", fg(SUBTLE)),
            Span::styled("Esc",  Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel",      fg(SUBTLE)),
        ]),
        Mode::ViewHours { .. } => Line::from(vec![
            Span::styled("j k",  Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" move  ",  fg(SUBTLE)),
            Span::styled("a",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" add  ",   fg(SUBTLE)),
            Span::styled("e",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" edit  ",  fg(SUBTLE)),
            Span::styled("d",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" delete  ", fg(SUBTLE)),
            Span::styled("Esc",  Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" back",    fg(SUBTLE)),
        ]),
        Mode::EditHours { .. } => Line::from(vec![
            Span::styled("Enter", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save  ", fg(SUBTLE)),
            Span::styled("Esc",   Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel", fg(SUBTLE)),
        ]),
        Mode::RepairProject { .. } => Line::from(vec![
            Span::styled("← →",  Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cycle  ", fg(SUBTLE)),
            Span::styled("Enter", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save  ", fg(SUBTLE)),
            Span::styled("Esc",   Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel", fg(SUBTLE)),
        ]),
        Mode::NewClient(_) | Mode::NewProject(_) | Mode::NewContact(_) => Line::from(vec![
            Span::styled("Tab",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" next field  ", fg(SUBTLE)),
            Span::styled("Ctrl+s", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save  ",  fg(SUBTLE)),
            Span::styled("Esc",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel",   fg(SUBTLE)),
        ]),
        Mode::ConfirmDelete { name, warning, .. } => {
            let mut spans = vec![
                Span::styled("delete ".to_string(),        fg(AMBER)),
                Span::styled(format!("'{name}'"),   bold_fg(ACCENT)),
                Span::styled("?  ".to_string(),            fg(AMBER)),
            ];
            if let Some(w) = warning {
                spans.push(Span::styled(format!("{w}  "), fg(AMBER)));
            }
            spans.push(Span::styled("y",  Style::default().fg(RED).bg(BORDER)));
            spans.push(Span::styled(" confirm  ".to_string(), fg(SUBTLE)));
            spans.push(Span::styled("n",  Style::default().fg(ACCENT).bg(BORDER)));
            spans.push(Span::styled(" cancel".to_string(),    fg(SUBTLE)));
            Line::from(spans)
        }
    };
    frame.render_widget(
        Paragraph::new(middle).style(Style::default().bg(CMDBAR_BG)),
        cols[1],
    );

    // Status right
    let hb = if app.config.animations { heartbeat_color(app.tick_count) } else { BRAND };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("● ".to_string(), fg(hb)),
            Span::styled("online · files · ok", fg(SUBTLE)),
        ])).style(Style::default().bg(CMDBAR_BG)),
        cols[2],
    );
}
