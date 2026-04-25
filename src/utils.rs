use ratatui::widgets::ListState;

pub fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

pub fn parse_hours_from_message(msg: &str) -> Option<f64> {
    msg.strip_prefix('+')
        .and_then(|s| s.split(' ').next())
        .and_then(|s| s.parse::<f64>().ok())
}

pub fn step_list(state: &mut ListState, len: usize, delta: i64) {
    if len == 0 { return; }
    let cur = state.selected().unwrap_or(0) as i64;
    let next = (cur + delta).clamp(0, len as i64 - 1) as usize;
    state.select(Some(next));
}
