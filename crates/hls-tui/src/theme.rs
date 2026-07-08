pub(crate) const PANEL_WIDTH: usize = 106;

pub(crate) fn top_border() -> String {
    format!("╭{}╮\n", "─".repeat(PANEL_WIDTH - 2))
}

pub(crate) fn divider() -> String {
    format!("├{}┤\n", "─".repeat(PANEL_WIDTH - 2))
}

pub(crate) fn bottom_border() -> String {
    format!("╰{}╯\n", "─".repeat(PANEL_WIDTH - 2))
}

pub(crate) fn panel_line(left: &str, body: &str, right: &str) -> String {
    let inner_width = PANEL_WIDTH - 4;
    let right_block = format!(" {right}");
    let right_width = char_count(&right_block);
    let body_width = inner_width.saturating_sub(right_width);
    let body_text = truncate_chars(&format!("{left:<10} {body}"), body_width);
    let padding = body_width.saturating_sub(char_count(&body_text));

    format!("│ {body_text}{}{right_block} │\n", " ".repeat(padding))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if char_count(value) <= max_chars {
        return value.to_owned();
    }

    let keep = max_chars.saturating_sub(1);
    let mut output: String = value.chars().take(keep).collect();
    output.push('…');
    output
}

fn char_count(value: &str) -> usize {
    value.chars().count()
}
