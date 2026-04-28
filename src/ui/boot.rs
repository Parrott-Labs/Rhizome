use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::{App, BootPhase};
use crate::theme::*;
use crate::ui::widgets::style_logo_line;

const LOGO: &[&str] = &[
    r"  ·         ●────·           ◦         ·──●        ◦       ·",
    r"   ╲       ╱ ╲    ╲         ╱ ╲       ╱    ╲      ╱ ╲     ╱",
    r"    ●─────●   ◦    ●───────●   ●─────●      ●────●   ◦───●",
    r"    │     │                │   │     │      │    │       │",
    r"────R─────H─────────I──────Z───O─────M──────E────·───────◦──────",
    r"    │    ╱ ╲       ╱╲      │  ╱╲    ╱ ╲     │    ╱ ╲",
    r"    ●───●   ●─────●  ●─────●─●  ●──●   ●────●───●   ●─·",
    r"       ╱     ╲   ╱    ╲   ╱ ╲    ╲   ╱       ╲   ╲",
    r"      ·       ●─·      ◦─●   ●────·─●         ◦   ●──·",
];

pub fn render_boot(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // Blank padding before logo
    lines.push(Line::default());
    lines.push(Line::default());

    // Logo lines
    for logo_line in LOGO {
        lines.push(style_logo_line(logo_line));
    }

    // Tagline — computed from real data
    let nc = app.clients.len();
    let np = app.projects.len();
    lines.push(Line::from(Span::styled(
        format!("     rhizome · local node · v0.1.0 · {np} edge{} · {nc} root{}",
            if np == 1 { "" } else { "s" },
            if nc == 1 { "" } else { "s" }),
        fg(SUBTLE),
    )));
    lines.push(Line::default());

    // Boot log lines (shown progressively)
    let show_count = match app.boot_phase {
        BootPhase::Logo  => 0,
        BootPhase::Lines => app.boot_lines_shown,
        BootPhase::Done  => app.boot_lines.len(),
    };

    for (tag, msg) in app.boot_lines.iter().take(show_count) {
        lines.push(boot_log_line(tag, msg));
    }

    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(BG).fg(TEXT))
        .block(Block::default().style(Style::default().bg(BG)));
    frame.render_widget(para, area);
}

fn boot_log_line<'a>(tag: &str, msg: &str) -> Line<'static> {
    let (tag_str, tag_color) = match tag {
        "ok"   => ("  ok  ", GREEN),
        "...." => (" .... ", AMBER),
        "boot" => (" boot ", BRAND),
        _      => ("      ", SUBTLE),
    };

    Line::from(vec![
        Span::styled("[ ".to_string(),          fg(SUBTLE)),
        Span::styled(tag_str.to_string(),        fg(tag_color)),
        Span::styled(" ] ".to_string(),          fg(SUBTLE)),
        Span::styled(msg.to_string(),            fg(TEXT)),
    ])
}
