use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{List, ListItem, Paragraph},
};

use crate::app::App;
use crate::models::{Client, Project, ProjectStatus};
use crate::theme::*;
use crate::ui::widgets::{col, progress_bar_line};

pub fn render_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // stats
            Constraint::Length(1), // broken-project warning
            Constraint::Length(1), // follow-up warning
            Constraint::Fill(1),   // body
        ])
        .split(area);

    render_stats(frame, app, v[0]);
    render_broken_warning(frame, app, v[1]);
    render_followup_warning(frame, app, v[2]);
    render_body(frame, app, v[3]);
}

fn render_broken_warning(frame: &mut Frame, app: &App, area: Rect) {
    if app.broken_projects.is_empty() {
        return;
    }

    let mut spans: Vec<Span> = vec![Span::styled(
        "✗ broken  ",
        Style::default().fg(RED).add_modifier(Modifier::BOLD),
    )];
    for (i, bp) in app.broken_projects.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", fg(MUTED)));
        }
        spans.push(Span::styled(bp.name.clone(), fg(ACCENT)));
        if !bp.bad_status.is_empty() {
            spans.push(Span::styled(
                format!(" (\"{}\" unrecognised)", bp.bad_status),
                fg(RED),
            ));
        }
    }
    spans.push(Span::styled("  · press ", fg(MUTED)));
    spans.push(Span::styled("r", Style::default().fg(ACCENT).bg(BORDER)));
    spans.push(Span::styled(" to repair", fg(MUTED)));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_followup_warning(frame: &mut Frame, app: &App, area: Rect) {
    let stale = app.stale_clients();
    if stale.is_empty() {
        return;
    }

    let mut spans: Vec<Span> = vec![Span::styled(
        "! follow-up  ",
        Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
    )];
    for (i, (client, days)) in stale.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", fg(MUTED)));
        }
        spans.push(Span::styled(client.name.clone(), fg(TEXT)));
        spans.push(Span::styled(format!(" ({}d)", days), fg(AMBER)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_stats(frame: &mut Frame, app: &App, area: Rect) {
    let cells = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let s = &app.stats;
    let client_delta = if s.clients_this_quarter > 0 {
        format!("+{} this quarter", s.clients_this_quarter)
    } else {
        String::new()
    };
    let project_delta = if s.total_projects > 0 {
        format!("of {} total", s.total_projects)
    } else {
        String::new()
    };
    let attn_delta = if s.attention_count > 0 {
        "need follow-up".into()
    } else {
        String::new()
    };
    let hours_val = if s.hours_this_week > 0.0 {
        format!("{:.1}h", s.hours_this_week)
    } else {
        "—".into()
    };
    render_stat_cell(
        frame,
        cells[0],
        "CLIENTS",
        &s.total_clients.to_string(),
        &client_delta,
        GREEN,
        false,
    );
    render_stat_cell(
        frame,
        cells[1],
        "ACTIVE PROJECTS",
        &s.active_projects.to_string(),
        &project_delta,
        SUBTLE,
        false,
    );
    render_stat_cell(
        frame,
        cells[2],
        "HOURS LOGGED",
        &hours_val,
        "",
        SUBTLE,
        false,
    );
    render_stat_cell(
        frame,
        cells[3],
        "ATTENTION",
        &s.attention_count.to_string(),
        &attn_delta,
        AMBER,
        s.attention_count > 0,
    );
}

fn render_stat_cell(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    delta: &str,
    delta_color: ratatui::style::Color,
    value_amber: bool,
) {
    let val_color = if value_amber { AMBER } else { ACCENT };
    let lines = vec![
        Line::from(Span::styled(
            label.to_string(),
            Style::default().fg(SUBTLE).add_modifier(Modifier::DIM),
        )),
        Line::from(Span::styled(
            value.to_string(),
            Style::default().fg(val_color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(delta.to_string(), fg(delta_color))),
    ];
    let para = Paragraph::new(Text::from(lines));
    frame.render_widget(para, area);
}

fn render_body(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_left(frame, app, cols[0]);
    render_right(frame, app, cols[1]);
}

// ── Left column ──────────────────────────────────────────────────────────────

fn render_left(frame: &mut Frame, app: &App, area: Rect) {
    // Recent activity header + 5 rows + blank + hours header + bars
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // section header
            Constraint::Length(5), // project list (5 items)
            Constraint::Length(1), // spacer
            Constraint::Length(1), // hours header
            Constraint::Fill(1),   // bars
        ])
        .split(area);

    // Section header
    frame.render_widget(
        Paragraph::new(Span::styled("recent activity", fg(SUBTLE))),
        rows[0],
    );

    // Project list (5 most-recent)
    let selected = app.dashboard_list.selected().unwrap_or(0);
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, p)| project_list_item(i, p, i == selected))
        .collect();
    let list = List::new(items)
        .highlight_style(Style::default().bg(SELECTED_BG))
        .highlight_symbol("");
    let mut state = app.dashboard_list.clone();
    frame.render_stateful_widget(list, rows[1], &mut state);

    // Hours distribution header
    frame.render_widget(
        Paragraph::new(Span::styled("week · hours distribution", fg(SUBTLE))),
        rows[3],
    );

    // Per-client hour bars
    render_hours_bars(frame, app, rows[4]);
}

fn render_hours_bars(frame: &mut Frame, app: &App, area: Rect) {
    // Aggregate spent hours per client, pick top clients
    let mut client_hours: Vec<(String, f64)> = app
        .clients
        .iter()
        .map(|c| {
            let h: f64 = app
                .projects
                .iter()
                .filter(|p| p.client_id == c.id && p.status != ProjectStatus::Archived)
                .map(|p| p.spent_hours)
                .sum();
            (c.name.clone(), h)
        })
        .filter(|(_, h)| *h > 0.0)
        .collect();
    client_hours.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let max_h = client_hours.first().map(|(_, h)| *h).unwrap_or(1.0);
    let bar_w = area.width as usize;

    let lines: Vec<Line> = client_hours
        .iter()
        .take(area.height as usize)
        .map(|(name, hours)| {
            progress_bar_line(name, hours / max_h, &format!("{:.1}h", hours), bar_w, false)
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

// ── Right column ──────────────────────────────────────────────────────────────

fn render_right(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Fill(1),   // graph
            Constraint::Length(1), // spacer
            Constraint::Length(1), // next up header
            Constraint::Length(4), // next up items
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled("network · active rhizome", fg(SUBTLE))),
        rows[0],
    );

    render_graph(frame, app, rows[1]);

    frame.render_widget(
        Paragraph::new(Span::styled("next · up", fg(SUBTLE))),
        rows[3],
    );

    render_next_up(frame, app, rows[4]);
}

fn render_graph(frame: &mut Frame, app: &App, area: Rect) {
    let lines = build_graph_lines(&app.clients, &app.projects);
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn build_graph_lines(clients: &[Client], projects: &[Project]) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("≋", bold_fg(BRAND)),
        Span::styled(" rhizome", bold_fg(BRAND)),
    ]));
    lines.push(Line::from(Span::styled("│", fg(MUTED))));

    for (ci, client) in clients.iter().enumerate() {
        let is_last = ci == clients.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };
        let cont = if is_last { "    " } else { "│   " };

        let client_projects: Vec<&Project> = projects
            .iter()
            .filter(|p| p.client_id == client.id && p.status != ProjectStatus::Archived)
            .collect();

        if client_projects.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(prefix.to_string(), fg(MUTED)),
                Span::styled("◉ ".to_string(), fg(ACCENT)),
                Span::styled(client.name.clone(), fg(ACCENT)),
            ]));
        } else if client_projects.len() == 1 {
            let p = client_projects[0];
            lines.push(Line::from(vec![
                Span::styled(prefix.to_string(), fg(MUTED)),
                Span::styled("◉ ".to_string(), fg(ACCENT)),
                Span::styled(col(&client.name, 22), fg(ACCENT)),
                Span::styled("──── ".to_string(), fg(MUTED)),
                Span::styled(col(&p.name, 12), fg(ACCENT)),
                Span::styled(" ──".to_string(), fg(MUTED)),
                Span::styled("● ".to_string(), fg(p.status_color())),
                Span::styled(p.status.to_string(), fg(p.status_color())),
            ]));
        } else {
            // First project on client line with branch
            let p0 = client_projects[0];
            lines.push(Line::from(vec![
                Span::styled(prefix.to_string(), fg(MUTED)),
                Span::styled("◉ ".to_string(), fg(ACCENT)),
                Span::styled(col(&client.name, 22), fg(ACCENT)),
                Span::styled("────┬── ".to_string(), fg(MUTED)),
                Span::styled(col(&p0.name, 12), fg(ACCENT)),
                Span::styled(" ──".to_string(), fg(MUTED)),
                Span::styled("● ".to_string(), fg(p0.status_color())),
                Span::styled(p0.status.to_string(), fg(p0.status_color())),
            ]));
            for (pi, p) in client_projects[1..].iter().enumerate() {
                let last_p = pi == client_projects.len() - 2;
                let pbranch = if last_p { "└── " } else { "├── " };
                lines.push(Line::from(vec![
                    Span::styled(format!("{}            {}", cont, pbranch), fg(MUTED)),
                    Span::styled(col(&p.name, 12), fg(ACCENT)),
                    Span::styled(" ──".to_string(), fg(MUTED)),
                    Span::styled("● ".to_string(), fg(p.status_color())),
                    Span::styled(p.status.to_string(), fg(p.status_color())),
                ]));
            }
        }

        if !is_last {
            lines.push(Line::from(Span::styled("│", fg(MUTED))));
        }
    }

    lines
}

fn render_next_up(frame: &mut Frame, app: &App, area: Rect) {
    // Build action items from upcoming (non-done) milestones
    let mut upcoming: Vec<(String, String)> = Vec::new();
    for project in &app.projects {
        if matches!(
            project.status,
            ProjectStatus::Archived | ProjectStatus::Lead
        ) {
            continue;
        }
        for ms in &project.milestones {
            if !ms.is_done() {
                let label = format!("{} · {}", project.name, ms.title);
                let due = ms.due_at.as_deref().unwrap_or("").to_string();
                upcoming.push((label, due));
                if upcoming.len() >= 4 {
                    break;
                }
            }
        }
        if upcoming.len() >= 4 {
            break;
        }
    }

    let lines: Vec<Line> = upcoming
        .iter()
        .enumerate()
        .map(|(i, (label, due))| {
            let (sym, sym_color) = if i == 0 {
                ("→", BLUE)
            } else {
                ("·", SUBTLE)
            };
            Line::from(vec![
                Span::styled(sym.to_string(), fg(sym_color)),
                Span::raw(" "),
                Span::styled(label.clone(), fg(TEXT)),
                Span::raw("  "),
                Span::styled(due.clone(), fg(SUBTLE)),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

// ── Project row (shared list item format) ─────────────────────────────────────

pub fn project_list_item(idx: usize, p: &Project, selected: bool) -> ListItem<'static> {
    let edge_span = if selected {
        Span::styled("│", fg(BRAND))
    } else {
        Span::raw(" ")
    };

    let idx_style = if selected { bold_fg(BRAND) } else { fg(MUTED) };
    let name_style = if selected { bold_fg(ACCENT) } else { fg(TEXT) };
    let bg = if selected { SELECTED_BG } else { BG };

    let spark = p.sparkline_str();

    let line = Line::from(vec![
        edge_span,
        Span::styled(format!("{:2} ", idx + 1), idx_style),
        Span::styled(col(&p.name, 14), name_style),
        Span::raw("  "),
        Span::styled(col(p.client_name.as_deref().unwrap_or("?"), 14), fg(SUBTLE)),
        Span::styled("● ".to_string(), fg(p.status_color())),
        Span::styled(format!("{:<10}", p.status), fg(p.status_color())),
        Span::styled(format!("{:>6.1}h ", p.spent_hours), fg(SUBTLE)),
        Span::styled(spark, fg(BRAND)),
    ]);

    ListItem::new(line).style(Style::default().bg(bg))
}
