use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    widgets::List,
    widgets::ListItem,
    Frame,
};

use crate::app::{App, Mode, NewProjectForm};
use crate::models::{ActivityEntry, Client, Milestone, Project};
use crate::theme::*;
use crate::ui::widgets::{col, progress_bar_line};
use crate::utils::parse_hours_from_message;

pub fn render_projects(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_project_list(frame, app, cols[0]);

    match &app.mode {
        Mode::NewProject(form) => render_new_project_form(frame, form, &app.clients, cols[1]),
        Mode::AddHours { project_id, input, error } => {
            if let Some(project) = app.selected_project() {
                render_project_detail(frame, project, cols[1]);
            }
            render_add_hours_popup(frame, project_id, input, error.as_deref(), cols[1]);
        }
        Mode::ViewHours { project_id } => {
            let entries = app.project_hours(project_id);
            let project_name = app.projects.iter()
                .find(|p| p.id == *project_id)
                .map(|p| p.name.as_str())
                .unwrap_or("?");
            let mut state = app.hours_list.clone();
            render_hours_list(frame, project_name, &entries, &mut state, cols[1]);
        }
        Mode::EditHours { project_id, activity_id: _, date_input, hours_input, focused, error } => {
            let entries = app.project_hours(project_id);
            let project_name = app.projects.iter()
                .find(|p| p.id == *project_id)
                .map(|p| p.name.as_str())
                .unwrap_or("?");
            let mut state = app.hours_list.clone();
            render_hours_list(frame, project_name, &entries, &mut state, cols[1]);
            render_edit_hours_popup(frame, date_input, hours_input, *focused, error.as_deref(), cols[1]);
        }
        _ => {
            if let Some(project) = app.selected_project() {
                render_project_detail(frame, project, cols[1]);
            }
        }
    }
}

// ── Project list ──────────────────────────────────────────────────────────────

fn render_project_list(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Fill(1)])
        .split(area);

    // Header
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("   # ", fg(MUTED)),
            Span::styled(format!("{:<16}", "PROJECT"),  Style::default().fg(SUBTLE).add_modifier(Modifier::DIM)),
            Span::raw("  "),
            Span::styled(format!("{:<14}", "CLIENT"),   Style::default().fg(SUBTLE).add_modifier(Modifier::DIM)),
            Span::styled(format!("{:<12}", "STATUS"),   Style::default().fg(SUBTLE).add_modifier(Modifier::DIM)),
            Span::styled(format!("{:>7}", "HOURS"),     Style::default().fg(SUBTLE).add_modifier(Modifier::DIM)),
            Span::styled(" PULSE",                      Style::default().fg(SUBTLE).add_modifier(Modifier::DIM)),
        ])),
        layout[0],
    );

    let sel = app.project_list.selected().unwrap_or(0);
    let items: Vec<ListItem> = app.projects.iter().enumerate()
        .map(|(i, p)| project_list_item(i, p, i == sel))
        .collect();
    let mut state = app.project_list.clone();

    let list = List::new(items)
        .highlight_style(Style::default().bg(SELECTED_BG))
        .highlight_symbol("");
    frame.render_stateful_widget(list, layout[1], &mut state);
}

fn project_list_item(idx: usize, p: &Project, selected: bool) -> ListItem<'static> {
    let edge       = if selected { Span::styled("│", fg(BRAND)) } else { Span::raw(" ") };
    let idx_style  = if selected { bold_fg(BRAND)  } else { fg(MUTED) };
    let name_style = if selected { bold_fg(ACCENT) } else { fg(TEXT)  };
    let bg         = if selected { SELECTED_BG } else { BG };

    let client = p.client_name.as_deref().unwrap_or("?");
    let spark   = p.sparkline_str();

    let line = Line::from(vec![
        edge,
        Span::styled(format!("{:2} ", idx + 1), idx_style),
        Span::styled(col(&p.name, 16),  name_style),
        Span::raw("  "),
        Span::styled(col(client, 14),   fg(SUBTLE)),
        Span::styled("● ".to_string(),              fg(p.status_color())),
        Span::styled(format!("{:<10}", p.status),   fg(p.status_color())),
        Span::styled(format!("{:>6.1}h", p.spent_hours), fg(SUBTLE)),
        Span::raw(" "),
        Span::styled(spark, fg(BRAND)),
    ]);

    ListItem::new(line).style(Style::default().bg(bg))
}

// ── Project detail ────────────────────────────────────────────────────────────

fn render_project_detail(frame: &mut Frame, p: &Project, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // "PROJECT" label
            Constraint::Length(1),  // name + summary
            Constraint::Length(1),  // meta line
            Constraint::Length(1),  // spacer
            Constraint::Length(7),  // key-value block
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // "Burn" label
            Constraint::Length(2),  // progress bars
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // "Milestones" label
            Constraint::Fill(1),    // milestone list
        ])
        .split(area);

    // Label
    frame.render_widget(
        Paragraph::new(Span::styled("PROJECT", Style::default().fg(SUBTLE).add_modifier(Modifier::DIM))),
        layout[0],
    );

    // Name + summary
    let summary = p.summary.as_deref().unwrap_or("");
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(p.name.clone(), bold_fg(ACCENT)),
            if !summary.is_empty() {
                Span::styled(format!(" — {}", summary), fg(SUBTLE))
            } else {
                Span::raw("")
            },
        ])),
        layout[1],
    );

    // Meta line
    let client_name = p.client_name.as_deref().unwrap_or("?");
    let started = &p.created_at[..10];
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(client_name.to_string(), fg(TEXT)),
            Span::styled(" · started ".to_string(), fg(MUTED)),
            Span::styled(started.to_string(), fg(SUBTLE)),
            Span::styled(" · ".to_string(), fg(MUTED)),
            Span::styled("● ".to_string(), fg(p.status_color())),
            Span::styled(p.status.to_string(), fg(p.status_color())),
        ])),
        layout[2],
    );

    // Key-value
    render_project_kv(frame, p, layout[4]);

    // Burn label
    frame.render_widget(
        Paragraph::new(Span::styled("Burn", fg(SUBTLE))),
        layout[6],
    );

    // Progress bars
    render_burn_bars(frame, p, layout[7]);

    // Milestones label
    frame.render_widget(
        Paragraph::new(Span::styled("Milestones", fg(SUBTLE))),
        layout[9],
    );

    // Milestone list
    render_milestones(frame, &p.milestones, layout[10]);
}

fn render_project_kv(frame: &mut Frame, p: &Project, area: Rect) {
    let lw = 12usize;
    let mut lines = Vec::new();

    macro_rules! kv {
        ($k:expr, $v:expr, $c:expr) => {
            Line::from(vec![
                Span::styled(format!("{:<lw$}", $k), fg(SUBTLE)),
                Span::styled($v, fg($c)),
            ])
        };
    }

    if let Some(b) = p.budget_hours {
        let unit = if p.budget_kind == "monthly" { "h / mo" } else { "h total" };
        lines.push(kv!("budget", format!("{} {}", b, unit), CYAN));
    }

    let spent_pct = p.spent_pct().map(|f| format!("  ({:.0}%)", f * 100.0)).unwrap_or_default();
    lines.push(Line::from(vec![
        Span::styled(format!("{:<lw$}", "spent"), fg(SUBTLE)),
        Span::styled(format!("{:.1} h", p.spent_hours), fg(CYAN)),
        Span::styled(spent_pct, fg(SUBTLE)),
    ]));

    if let Some(r) = p.rate_cents {
        lines.push(kv!("rate", format!("€ {} / h", r / 100), CYAN));
    }

    if !p.deadline_display().is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<lw$}", "deadline"), fg(SUBTLE)),
            Span::styled(p.deadline_display(), fg(ACCENT)),
        ]));
    }

    if let Some(owner) = &p.owner {
        lines.push(kv!("owner", owner.clone(), TEXT));
    }

    if let Some(repo) = &p.repo_url {
        lines.push(kv!("repo", repo.clone(), BRAND));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_burn_bars(frame: &mut Frame, p: &Project, area: Rect) {
    let w = area.width as usize;
    let mut lines = Vec::new();

    if let Some(budget) = p.budget_hours {
        let frac = if budget > 0.0 { p.spent_hours / budget } else { 0.0 };
        let display = format!("{:.1}/{:.0}h", p.spent_hours, budget);
        lines.push(progress_bar_line("hours", frac, &display, w, frac >= 1.0));
    }

    let done = p.milestones.iter().filter(|m| m.is_done()).count();
    let total = p.milestones.len();
    if total > 0 {
        let frac = done as f64 / total as f64;
        let display = format!("{}/{}", done, total);
        lines.push(progress_bar_line("milestones", frac, &display, w, frac >= 1.0));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_new_project_form(frame: &mut Frame, form: &NewProjectForm, clients: &[Client], area: Rect) {
    const LW: usize = 12;
    const VW: usize = 28;

    let mut lines: Vec<Line> = Vec::new();

    let title = if form.edit_id.is_some() { "EDIT PROJECT" } else { "NEW PROJECT" };
    lines.push(Line::from(Span::styled(
        title,
        Style::default().fg(SUBTLE).add_modifier(Modifier::DIM),
    )));
    lines.push(Line::default());

    for (i, field) in form.fields.iter().enumerate() {
        let focused = i == form.focused;

        let label_style = if field.required {
            Style::default().fg(ACCENT)
        } else {
            fg(SUBTLE)
        };

        let (val_fg, val_bg) = if focused { (TEXT, SURFACE) } else { (SUBTLE, BG) };

        // Status selector — spinner (field 3)
        let line = if i == 3 {
            let (val_fg, val_bg) = if focused { (TEXT, SURFACE) } else { (SUBTLE, BG) };
            let display = if focused {
                format!("◀ {} ▶", field.value)
            } else {
                format!("  {}  ", field.value)
            };
            let mut spans = vec![
                Span::styled(format!("  {:<LW$}", field.label), label_style),
                Span::styled(display, Style::default().fg(val_fg).bg(val_bg)),
            ];
            if focused {
                spans.push(Span::styled("  ←/→ cycle", fg(MUTED)));
            }
            Line::from(spans)
        // Budget kind selector — cycling pills (field 4)
        } else if i == 4 {
            let mut spans = vec![
                Span::styled(format!("  {:<LW$}", field.label), label_style),
            ];
            for &kind in crate::app::BUDGET_KINDS {
                let selected = field.value == kind;
                let (pill_fg, pill_bg) = if selected { (BG, ACCENT) } else { (MUTED, BG) };
                spans.push(Span::styled(format!("[{}]", kind), Style::default().fg(pill_fg).bg(pill_bg)));
            }
            if focused {
                spans.push(Span::styled("  ← →", fg(SUBTLE)));
            }
            Line::from(spans)
        // Client field gets ghost-text completion
        } else if i == 1 && focused {
            let typed_chars: Vec<char> = field.value.chars().collect();
            let ghost = client_ghost(&field.value, clients);
            let ghost_chars: Vec<char> = ghost.chars().collect();

            // How many typed chars fit before cursor + ghost overflow VW
            // Reserve 1 for cursor, then fill with ghost up to VW
            let typed_visible = VW.saturating_sub(1).min(typed_chars.len());
            let visible_typed: String = typed_chars[typed_chars.len() - typed_visible..].iter().collect();
            let ghost_budget = VW.saturating_sub(typed_visible + 1);
            let visible_ghost: String = ghost_chars.iter().take(ghost_budget).collect();
            let used = typed_visible + 1 + visible_ghost.chars().count();
            let pad = " ".repeat(VW.saturating_sub(used));

            Line::from(vec![
                Span::styled(format!("  {:<LW$}", field.label), label_style),
                Span::styled(visible_typed,  Style::default().fg(val_fg).bg(val_bg)),
                Span::styled("█",            Style::default().fg(ACCENT).bg(val_bg)),
                Span::styled(visible_ghost,  Style::default().fg(MUTED).bg(val_bg)),
                Span::styled(pad,            Style::default().bg(val_bg)),
            ])
        } else {
            let val_text = &field.value;
            let cursor   = if focused { "█" } else { "" };
            let padded   = format!("{val_text}{cursor}");
            Line::from(vec![
                Span::styled(format!("  {:<LW$}", field.label), label_style),
                Span::styled(
                    format!("{:<VW$}", tail_chars(&padded, VW)),
                    Style::default().fg(val_fg).bg(val_bg),
                ),
            ])
        };

        lines.push(line);
    }

    lines.push(Line::default());
    if let Some(err) = &form.error {
        lines.push(Line::from(Span::styled(format!("  ✗ {err}"), fg(RED))));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  Tab",    Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" complete / next  ", fg(SUBTLE)),
            Span::styled("Ctrl+s",   Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save  ",  fg(SUBTLE)),
            Span::styled("Esc",      Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel",  fg(SUBTLE)),
        ]));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_add_hours_popup(frame: &mut Frame, _project_id: &str, input: &str, error: Option<&str>, area: Rect) {
    // Center a small popup in the given area
    let popup_h = 7u16;
    let popup_w = 36u16;
    let x = area.x + area.width.saturating_sub(popup_w) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect {
        x,
        y,
        width:  popup_w.min(area.width),
        height: popup_h.min(area.height),
    };

    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BRAND))
            .title(Span::styled(" add hours ", Style::default().fg(ACCENT))),
        popup_area,
    );

    let inner = Rect {
        x:      popup_area.x + 2,
        y:      popup_area.y + 1,
        width:  popup_area.width.saturating_sub(4),
        height: popup_area.height.saturating_sub(2),
    };

    let cursor = format!("{}█", input);
    let padded  = format!("{:<8}", cursor);

    let bottom = if let Some(err) = error {
        Line::from(Span::styled(format!("✗ {}", err), fg(RED)))
    } else {
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save  ", fg(SUBTLE)),
            Span::styled("Esc", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel", fg(SUBTLE)),
        ])
    };

    let lines = vec![
        Line::default(),
        Line::from(vec![
            Span::styled(format!("{:<10}", "hours"), fg(SUBTLE)),
            Span::styled(padded, Style::default().fg(TEXT).bg(SURFACE)),
        ]),
        Line::default(),
        bottom,
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines)).alignment(Alignment::Left),
        inner,
    );
}

fn render_hours_list(
    frame: &mut Frame,
    project_name: &str,
    entries: &[&ActivityEntry],
    state: &mut ratatui::widgets::ListState,
    area: Rect,
) {
    use ratatui::widgets::{List, ListItem};

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Fill(1)])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled("HOURS LOG", Style::default().fg(SUBTLE).add_modifier(Modifier::DIM))),
        layout[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(project_name.to_string(), bold_fg(ACCENT)),
            Span::styled(
                format!("  {} entr{}", entries.len(), if entries.len() == 1 { "y" } else { "ies" }),
                fg(SUBTLE),
            ),
        ])),
        layout[1],
    );

    let sel = state.selected().unwrap_or(usize::MAX);
    let items: Vec<ListItem> = entries.iter().enumerate().map(|(i, e)| {
        let selected = i == sel;
        let edge = if selected { Span::styled("│", fg(BRAND)) } else { Span::raw(" ") };
        let h = e.hours
            .unwrap_or_else(|| parse_hours_from_message(&e.message).unwrap_or(0.0));
        let date = if e.at.len() >= 10 { &e.at[..10] } else { &e.at };
        let bg = if selected { SELECTED_BG } else { BG };

        let line = Line::from(vec![
            edge,
            Span::styled(format!("{:<12}", date),         if selected { bold_fg(ACCENT) } else { fg(TEXT)   }),
            Span::styled(format!("{:>6.1}h", h),          if selected { bold_fg(BRAND)  } else { fg(CYAN)   }),
            Span::raw("  "),
            Span::styled(col(&e.message, 28),             fg(SUBTLE)),
        ]);
        ListItem::new(line).style(Style::default().bg(bg))
    }).collect();

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("no hours logged yet · press a to add", fg(MUTED))),
            layout[2],
        );
    } else {
        let list = List::new(items)
            .highlight_style(Style::default().bg(SELECTED_BG))
            .highlight_symbol("");
        frame.render_stateful_widget(list, layout[2], state);
    }
}

fn render_edit_hours_popup(frame: &mut Frame, date_input: &str, hours_input: &str, focused: usize, error: Option<&str>, area: Rect) {
    let popup_h = 8u16;
    let popup_w = 36u16;
    let x = area.x + area.width.saturating_sub(popup_w) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect {
        x,
        y,
        width:  popup_w.min(area.width),
        height: popup_h.min(area.height),
    };

    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(CYAN))
            .title(Span::styled(" edit hours ", Style::default().fg(ACCENT))),
        popup_area,
    );

    let inner = Rect {
        x:      popup_area.x + 2,
        y:      popup_area.y + 1,
        width:  popup_area.width.saturating_sub(4),
        height: popup_area.height.saturating_sub(2),
    };

    let field = |label: &'static str, value: &str, active: bool| -> Line<'static> {
        let (fg_c, bg_c) = if active { (TEXT, SURFACE) } else { (SUBTLE, BG) };
        let cursor = if active { format!("{}█", value) } else { value.to_string() };
        Line::from(vec![
            Span::styled(format!("{:<10}", label), fg(SUBTLE)),
            Span::styled(format!("{:<14}", cursor), Style::default().fg(fg_c).bg(bg_c)),
        ])
    };

    let bottom = if let Some(err) = error {
        Line::from(Span::styled(format!("✗ {}", err), fg(RED)))
    } else {
        Line::from(vec![
            Span::styled("Tab",   Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" next  ", fg(SUBTLE)),
            Span::styled("Enter", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save  ", fg(SUBTLE)),
            Span::styled("Esc",   Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel", fg(SUBTLE)),
        ])
    };

    let lines = vec![
        Line::default(),
        field("date",  date_input,  focused == 0),
        field("hours", hours_input, focused == 1),
        Line::default(),
        bottom,
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines)).alignment(Alignment::Left),
        inner,
    );
}

fn tail_chars(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= n {
        chars.into_iter().collect()
    } else {
        chars[chars.len() - n..].iter().collect()
    }
}

fn client_ghost(typed: &str, clients: &[Client]) -> String {
    if typed.is_empty() { return String::new(); }
    let lower = typed.to_lowercase();
    clients.iter()
        .find(|c| c.name.to_lowercase().starts_with(&lower))
        .map(|c| c.name[typed.len()..].to_string())
        .unwrap_or_default()
}

fn render_milestones(frame: &mut Frame, milestones: &[Milestone], area: Rect) {
    let mut sorted: Vec<&Milestone> = milestones.iter().collect();
    sorted.sort_by_key(|m| m.ord);

    let lines: Vec<Line> = sorted.iter().take(area.height as usize).map(|m| {
        let sym_style = Style::default().fg(m.symbol_color());
        let text_style = if m.is_done() { fg(SUBTLE) } else if m.done_at.is_none() {
            // Currently active: check if this is the first undone
            fg(TEXT)
        } else { fg(TEXT) };

        // Mark currently-active milestone in accent
        let title_style = if !m.is_done() && {
            sorted.iter().all(|prev| prev.ord < m.ord && !prev.is_done() || prev.ord >= m.ord)
        } {
            fg(ACCENT)
        } else {
            text_style
        };

        Line::from(vec![
            Span::styled(m.symbol().to_string(), sym_style),
            Span::raw(" "),
            Span::styled(format!("{:<42}", &m.title), title_style),
            Span::styled(m.due_display(), fg(SUBTLE)),
        ])
    }).collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}
