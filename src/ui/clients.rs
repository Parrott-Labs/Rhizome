use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::app::{App, Mode, NewClientForm, NewContactForm};
use crate::models::{CONTACT_KINDS, Client, ContactMoment};
use crate::theme::*;
use crate::ui::widgets::{col, relative_time};

pub fn render_clients(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_client_list(frame, app, cols[0]);

    match &app.mode {
        Mode::NewClient(form) => render_new_client_form(frame, form, cols[1]),
        Mode::NewContact(form) => render_new_contact_form(frame, form, cols[1]),
        Mode::ViewContacts => {
            if let Some(client) = app.selected_client() {
                let contacts = app.client_contacts(&client.id.clone());
                let mut state = app.contact_list.clone();
                render_contact_list_view(frame, client, &contacts, &mut state, cols[1]);
            }
        }
        _ => {
            if let Some(client) = app.selected_client() {
                let projects = app.client_projects(&client.id.clone());
                let contacts = app.client_contacts(&client.id.clone());
                render_client_detail(frame, client, &projects, &contacts, cols[1]);
            } else {
                render_empty_detail(frame, cols[1]);
            }
        }
    }
}

// ── Client list ──────────────────────────────────────────────────────────────

fn render_client_list(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Fill(1)])
        .split(area);

    // Header row
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("   # ", fg(MUTED)),
            Span::styled(
                format!("{:<22}", "NAME"),
                Style::default().fg(SUBTLE).add_modifier(Modifier::DIM),
            ),
            Span::styled(
                format!("{:<16}", "CITY"),
                Style::default().fg(SUBTLE).add_modifier(Modifier::DIM),
            ),
            Span::styled(
                format!("{:<5}", "PROJ"),
                Style::default().fg(SUBTLE).add_modifier(Modifier::DIM),
            ),
            Span::styled(
                format!("{:<12}", "LAST CONTACT"),
                Style::default().fg(SUBTLE).add_modifier(Modifier::DIM),
            ),
            Span::styled("●", Style::default().fg(SUBTLE).add_modifier(Modifier::DIM)),
        ])),
        layout[0],
    );

    let sel = app.client_list.selected().unwrap_or(0);
    let items: Vec<ListItem> = app
        .clients
        .iter()
        .enumerate()
        .map(|(i, c)| client_list_item(i, c, i == sel, &app.projects))
        .collect();
    let mut state = app.client_list.clone();

    let list = List::new(items)
        .highlight_style(Style::default().bg(SELECTED_BG))
        .highlight_symbol("");
    frame.render_stateful_widget(list, layout[1], &mut state);
}

fn client_list_item(
    idx: usize,
    c: &Client,
    selected: bool,
    all_projects: &[crate::models::Project],
) -> ListItem<'static> {
    let edge = if selected {
        Span::styled("│", fg(BRAND))
    } else {
        Span::raw(" ")
    };
    let idx_style = if selected { bold_fg(BRAND) } else { fg(MUTED) };
    let name_style = if selected { bold_fg(ACCENT) } else { fg(TEXT) };
    let bg = if selected { SELECTED_BG } else { BG };

    let proj_count = all_projects.iter().filter(|p| p.client_id == c.id).count();
    let last_contact = relative_time(&c.updated_at);
    let city = c.city.as_deref().unwrap_or("");

    let line = Line::from(vec![
        edge,
        Span::styled(format!("{:2} ", idx + 1), idx_style),
        Span::styled(col(&c.name, 22), name_style),
        Span::styled(col(city, 16), fg(SUBTLE)),
        Span::styled(format!("{:<5}", proj_count), fg(SUBTLE)),
        Span::styled(format!("{:<12}", last_contact), fg(SUBTLE)),
        Span::styled("●".to_string(), fg(c.health_color())),
    ]);

    ListItem::new(line).style(Style::default().bg(bg))
}

// ── Client detail ─────────────────────────────────────────────────────────────

fn render_client_detail(
    frame: &mut Frame,
    client: &Client,
    projects: &[&crate::models::Project],
    contacts: &[&ContactMoment],
    area: Rect,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // "CLIENT" label
            Constraint::Length(1), // name heading
            Constraint::Length(1), // meta line
            Constraint::Length(1), // spacer
            Constraint::Length(6), // key-value block
            Constraint::Length(1), // spacer
            Constraint::Length(1), // "Projects" label
            Constraint::Length(3), // mini project list
            Constraint::Length(1), // spacer
            Constraint::Length(1), // "Recent · log" label
            Constraint::Fill(1),   // activity
        ])
        .split(area);

    // "CLIENT" label
    frame.render_widget(
        Paragraph::new(Span::styled(
            "CLIENT",
            Style::default().fg(SUBTLE).add_modifier(Modifier::DIM),
        )),
        layout[0],
    );

    // Name heading
    frame.render_widget(
        Paragraph::new(Span::styled(client.name.clone(), bold_fg(ACCENT))),
        layout[1],
    );

    // Meta line
    let acct = client.acct_code.as_deref().unwrap_or("—");
    let since = &client.created_at[..7]; // "YYYY-MM"
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("acct ".to_string(), fg(SUBTLE)),
            Span::styled(acct.to_string(), fg(CYAN)),
            Span::styled(" · since ".to_string(), fg(MUTED)),
            Span::styled(since.to_string(), fg(SUBTLE)),
        ])),
        layout[2],
    );

    // Key-value block
    render_kv_block(frame, client, layout[4]);

    // Projects label
    frame.render_widget(
        Paragraph::new(Span::styled("Projects", fg(SUBTLE))),
        layout[6],
    );

    // Mini project list (up to 3 rows)
    render_mini_projects(frame, projects, layout[7]);

    // Contacts label
    frame.render_widget(
        Paragraph::new(Span::styled("contacts · log", fg(SUBTLE))),
        layout[9],
    );

    // Contact entries
    render_contacts(frame, contacts, layout[10]);
}

fn render_kv_block(frame: &mut Frame, client: &Client, area: Rect) {
    let label_w = 12usize;
    let mut lines = Vec::new();

    let kv = |k: &str, v: String, v_color: ratatui::style::Color| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("{:<label_w$}", k), fg(SUBTLE)),
            Span::styled(v, fg(v_color)),
        ])
    };

    lines.push(kv(
        "domain",
        client.domain.clone().unwrap_or_default(),
        ACCENT,
    ));
    lines.push(kv("location", client.location_str(), TEXT));
    lines.push({
        let contact = client.primary_contact.clone().unwrap_or_default();
        let role = client.primary_role.clone().unwrap_or_default();
        Line::from(vec![
            Span::styled(format!("{:<label_w$}", "primary"), fg(SUBTLE)),
            Span::styled(format!("{} ", contact), fg(TEXT)),
            Span::styled(format!("· {}", role), fg(SUBTLE)),
        ])
    });
    lines.push(kv("rate", client.rate_display(), CYAN));
    lines.push(kv("currency", client.currency.clone(), TEXT));

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_mini_projects(frame: &mut Frame, projects: &[&crate::models::Project], area: Rect) {
    let lines: Vec<Line> = projects
        .iter()
        .take(area.height as usize)
        .map(|p| {
            Line::from(vec![
                Span::styled("  ".to_string(), fg(MUTED)),
                Span::styled(col(&p.name, 16), fg(TEXT)),
                Span::raw("  "),
                Span::styled("● ".to_string(), fg(p.status_color())),
                Span::styled(p.status.to_string(), fg(p.status_color())),
                Span::raw("  "),
                Span::styled(format!("{:.1}h", p.spent_hours), fg(SUBTLE)),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_contacts(frame: &mut Frame, contacts: &[&ContactMoment], area: Rect) {
    if contacts.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  no contacts yet · press c to log one",
                fg(MUTED),
            )),
            area,
        );
        return;
    }

    let lines: Vec<Line> = contacts
        .iter()
        .take(area.height as usize)
        .map(|c| {
            Line::from(vec![
                Span::styled(c.date.clone(), fg(SUBTLE)),
                Span::raw("   "),
                Span::styled(c.kind_symbol().to_string(), fg(c.kind_color())),
                Span::raw(" "),
                Span::styled(format!("{:<8}", &c.kind), fg(c.kind_color())),
                Span::raw("  "),
                Span::styled(c.summary.clone(), fg(TEXT)),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_new_client_form(frame: &mut Frame, form: &NewClientForm, area: Rect) {
    const LW: usize = 12; // label column width
    const VW: usize = 28; // value column width

    let mut lines: Vec<Line> = Vec::new();

    let title = if form.edit_id.is_some() {
        "EDIT CLIENT"
    } else {
        "NEW CLIENT"
    };
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

        let val_text = &field.value;
        let cursor = if focused { "█" } else { "" };
        let padded = format!("{val_text}{cursor}");

        let (val_fg, val_bg) = if focused {
            (TEXT, SURFACE)
        } else {
            (SUBTLE, BG)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {:<LW$}", field.label), label_style),
            Span::styled(
                format!("{:<VW$}", tail_chars(&padded, VW)),
                Style::default().fg(val_fg).bg(val_bg),
            ),
        ]));
    }

    // Error or hint
    lines.push(Line::default());
    if let Some(err) = &form.error {
        lines.push(Line::from(Span::styled(format!("  ✗ {err}"), fg(RED))));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  Tab", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" next  ", fg(SUBTLE)),
            Span::styled("Ctrl+s", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save  ", fg(SUBTLE)),
            Span::styled("Esc", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel", fg(SUBTLE)),
        ]));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn tail_chars(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= n {
        chars.into_iter().collect()
    } else {
        chars[chars.len() - n..].iter().collect()
    }
}

fn render_contact_list_view(
    frame: &mut Frame,
    client: &Client,
    contacts: &[&ContactMoment],
    state: &mut ListState,
    area: Rect,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(area);

    // Header
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "CONTACTS · ",
                Style::default().fg(SUBTLE).add_modifier(Modifier::DIM),
            ),
            Span::styled(client.name.clone(), fg(ACCENT)),
        ])),
        layout[0],
    );

    // Contact list
    if contacts.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  no contacts yet · press n to add one",
                fg(MUTED),
            ))),
            layout[1],
        );
    } else {
        let selected = state.selected().unwrap_or(0);
        let items: Vec<ListItem> = contacts
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let sel = i == selected;
                let bg = if sel { SELECTED_BG } else { BG };
                let edge = if sel {
                    Span::styled("│", fg(BRAND))
                } else {
                    Span::raw(" ")
                };
                Line::from(vec![
                    edge,
                    Span::raw(" "),
                    Span::styled(
                        c.date.clone(),
                        if sel { bold_fg(ACCENT) } else { fg(SUBTLE) },
                    ),
                    Span::raw("   "),
                    Span::styled(c.kind_symbol().to_string(), fg(c.kind_color())),
                    Span::raw(" "),
                    Span::styled(format!("{:<8}", &c.kind), fg(c.kind_color())),
                    Span::raw("  "),
                    Span::styled(c.summary.clone(), if sel { fg(TEXT) } else { fg(SUBTLE) }),
                ]);
                ListItem::new(Line::from(vec![
                    if sel {
                        Span::styled("│", fg(BRAND))
                    } else {
                        Span::raw(" ")
                    },
                    Span::raw(" "),
                    Span::styled(
                        c.date.clone(),
                        if sel { bold_fg(ACCENT) } else { fg(SUBTLE) },
                    ),
                    Span::raw("   "),
                    Span::styled(c.kind_symbol().to_string(), fg(c.kind_color())),
                    Span::raw(" "),
                    Span::styled(format!("{:<8}", &c.kind), fg(c.kind_color())),
                    Span::raw("  "),
                    Span::styled(c.summary.clone(), if sel { fg(TEXT) } else { fg(SUBTLE) }),
                ]))
                .style(Style::default().bg(bg))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(SELECTED_BG))
            .highlight_symbol("");
        frame.render_stateful_widget(list, layout[1], state);
    }

    // Hints
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("n", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" new  ", fg(SUBTLE)),
            Span::styled("e", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" edit  ", fg(SUBTLE)),
            Span::styled("d", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" delete  ", fg(SUBTLE)),
            Span::styled("Esc", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" back", fg(SUBTLE)),
        ])),
        layout[2],
    );
}

fn render_new_contact_form(frame: &mut Frame, form: &NewContactForm, area: Rect) {
    const LW: usize = 10;
    const VW: usize = 28;

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(
            "NEW CONTACT · ",
            Style::default().fg(SUBTLE).add_modifier(Modifier::DIM),
        ),
        Span::styled(form.client_name.clone(), fg(ACCENT)),
    ]));
    lines.push(Line::default());

    for (i, field) in form.fields.iter().enumerate() {
        let focused = i == form.focused;
        let label_style = Style::default().fg(ACCENT); // all required
        let (val_fg, val_bg) = if focused {
            (TEXT, SURFACE)
        } else {
            (SUBTLE, BG)
        };

        // Type field: show cycling selector
        let line = if i == 1 {
            let spans: Vec<Span> = CONTACT_KINDS
                .iter()
                .map(|&k| {
                    let active = k == field.value.as_str();
                    if active && focused {
                        Span::styled(format!(" {k} "), Style::default().fg(BG).bg(ACCENT))
                    } else if active {
                        Span::styled(format!(" {k} "), Style::default().fg(ACCENT).bg(SURFACE))
                    } else {
                        Span::styled(format!(" {k} "), Style::default().fg(MUTED).bg(BG))
                    }
                })
                .collect();
            let mut row = vec![Span::styled(format!("  {:<LW$}", field.label), label_style)];
            row.extend(spans);
            if focused {
                row.push(Span::styled("  ←/→ cycle", fg(MUTED)));
            }
            Line::from(row)
        } else {
            let val_text = &field.value;
            let cursor = if focused { "█" } else { "" };
            let padded = format!("{val_text}{cursor}");
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
            Span::styled("  Tab", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" next  ", fg(SUBTLE)),
            Span::styled("Ctrl+s", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" save  ", fg(SUBTLE)),
            Span::styled("Esc", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled(" cancel", fg(SUBTLE)),
        ]));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_empty_detail(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::default(),
        Line::default(),
        Line::from(Span::styled("           no clients yet", fg(SUBTLE))),
        Line::default(),
        Line::from(vec![Span::styled("        ·─────●─────·", fg(MUTED))]),
        Line::from(vec![Span::styled("         ╲   ╱ ╲   ╱", fg(MUTED))]),
        Line::from(vec![
            Span::styled("          ●─●   ●─●", fg(MUTED)),
            Span::styled("           press  ", fg(SUBTLE)),
            Span::styled("n", Style::default().fg(ACCENT).bg(BORDER)),
            Span::styled("  to add one", fg(SUBTLE)),
        ]),
    ];
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}
