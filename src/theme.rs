use ratatui::style::{Color, Modifier, Style};

pub const BG:          Color = Color::Rgb(26,  26,  26);
pub const SURFACE:     Color = Color::Rgb(34,  34,  34);
pub const BORDER:      Color = Color::Rgb(46,  46,  46);
pub const MUTED:       Color = Color::Rgb(58,  58,  58);
pub const SUBTLE:      Color = Color::Rgb(85,  85,  85);
pub const TEXT:        Color = Color::Rgb(200, 196, 187);
pub const ACCENT:      Color = Color::Rgb(226, 221, 212);
pub const GREEN:       Color = Color::Rgb(78,  255, 154);
pub const AMBER:       Color = Color::Rgb(255, 179, 71);
pub const RED:         Color = Color::Rgb(204, 102, 102);
pub const BLUE:        Color = Color::Rgb(126, 184, 255);
pub const CYAN:        Color = Color::Rgb(93,  202, 165);
pub const BRAND:       Color = Color::Rgb(93,  202, 165);
pub const SELECTED_BG: Color = Color::Rgb(30,  36,  34);
pub const CMDBAR_BG:   Color = Color::Rgb(23,  23,  23);

pub fn fg(c: Color) -> Style { Style::default().fg(c) }
pub fn bold_fg(c: Color) -> Style { Style::default().fg(c).add_modifier(Modifier::BOLD) }

/// Heartbeat pulse: interpolate BRAND ↔ dim-brand over a 2.4 s sine cycle.
/// Call with app.tick_count (120 ms ticks → 20 ticks per cycle).
pub fn heartbeat_color(tick: u64) -> Color {
    let phase = (tick as f64 / 20.0) * 2.0 * std::f64::consts::PI;
    let opacity = phase.sin().mul_add(0.325, 0.675); // range 0.35..1.0
    let r = (93.0 * opacity) as u8;
    let g = (202.0 * opacity) as u8;
    let b = (165.0 * opacity) as u8;
    Color::Rgb(r, g, b)
}

pub fn status_color(status: &str) -> Color {
    match status {
        "running"  => BLUE,
        "active"   => GREEN,
        "blocked"  => AMBER,
        "pending"  => SUBTLE,
        "lead"     => SUBTLE,
        "archived" => SUBTLE,
        "error"    => RED,
        "warn"     => AMBER,
        "idle"     => SUBTLE,
        _          => TEXT,
    }
}
