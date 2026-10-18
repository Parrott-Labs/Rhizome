use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::theme::*;
use chrono::{DateTime, Utc};

/// Truncate `s` to at most `n` chars, then pad with spaces to exactly `n`.
pub fn col(s: &str, n: usize) -> String {
    let mut out: String = s.chars().take(n).collect();
    while out.chars().count() < n {
        out.push(' ');
    }
    out
}

// ── Progress bar line ─────────────────────────────────────────────────────────

/// Returns a Line for a progress bar row.
/// `filled_frac` is 0.0..=1.0. `display` is the right-aligned value string.
pub fn progress_bar_line(
    name: &str,
    filled_frac: f64,
    display: &str,
    bar_width: usize,
    complete: bool,
) -> Line<'static> {
    let name_width = 18usize;
    let val_width = 8usize;
    let bar_w = bar_width.saturating_sub(name_width + val_width + 2).max(4);

    let filled = ((filled_frac.clamp(0.0, 1.0)) * bar_w as f64).round() as usize;
    let empty = bar_w.saturating_sub(filled);

    let filled_str: String = std::iter::repeat_n('█', filled).collect();
    let empty_str: String = std::iter::repeat_n('░', empty).collect();

    let bar_color = if complete { GREEN } else { BRAND };

    let name_s = col(name, 18);

    Line::from(vec![
        Span::styled(name_s, fg(TEXT)),
        Span::raw(" "),
        Span::styled(filled_str, fg(bar_color)),
        Span::styled(empty_str, fg(MUTED)),
        Span::raw(" "),
        Span::styled(format!("{:>8}", display), fg(CYAN)),
    ])
}


// ── Boot logo styling ─────────────────────────────────────────────────────────

pub fn style_logo_line(line: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for ch in line.chars() {
        let style = match ch {
            'R' | 'H' | 'I' | 'Z' | 'O' | 'M' | 'E' => {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            }
            '●' | '◦' => Style::default().fg(BRAND),
            '·' => Style::default().fg(BRAND),
            _ => Style::default().fg(MUTED),
        };
        spans.push(Span::styled(ch.to_string(), style));
    }
    Line::from(spans)
}

// ── Relative time display ────────────────────────────────────────────────────

pub fn relative_time(iso: &str) -> String {
    let Ok(dt) = iso.parse::<DateTime<Utc>>() else {
        return iso.to_string();
    };
    let now = Utc::now();
    let secs = (now - dt).num_seconds();
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}min", s / 60),
        s if s < 86400 => format!("{}h", s / 3600),
        s if s < 172800 => "yesterday".into(),
        s => format!("{}d", s / 86400),
    }
}
