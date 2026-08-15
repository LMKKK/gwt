use crate::{status, theme::Theme, types::Worktree};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
const HEADERS: [&str; 5] = ["", "BRANCH", "PATH", "COMMIT", "STATUS"];
fn width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}
pub fn truncate(value: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if width(value) <= max {
        return value.into();
    }
    if max == 1 {
        return "…".into();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in value.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > max - 1 {
            break;
        }
        out.push(c);
        used += w
    }
    out.push('…');
    out
}
fn pad(value: &str, target: usize) -> String {
    format!(
        "{}{}",
        value,
        " ".repeat(target.saturating_sub(width(value)))
    )
}
pub fn render(
    worktrees: &[Worktree],
    terminal_width: usize,
    color: bool,
    selected: Option<usize>,
) -> Vec<String> {
    let rows: Vec<[String; 5]> = worktrees
        .iter()
        .map(|w| {
            [
                if w.current { "*" } else { "" }.into(),
                w.branch.clone().unwrap_or_else(|| "detached".into()),
                w.path.clone(),
                if w.head.is_empty() {
                    "-".into()
                } else {
                    w.head.chars().take(12).collect()
                },
                status::format(w),
            ]
        })
        .collect();
    let mut widths = [0; 5];
    for (i, h) in HEADERS.iter().enumerate() {
        widths[i] = width(h)
    }
    for row in &rows {
        for i in 0..5 {
            widths[i] = widths[i].max(width(&row[i]))
        }
    }
    let mins = [1, 3, 3, 4, 3];
    while widths.iter().sum::<usize>() + 8 > terminal_width {
        let Some((i, _)) = widths
            .iter()
            .enumerate()
            .filter(|(i, w)| **w > mins[*i])
            .max_by_key(|(i, w)| **w - mins[*i])
        else {
            break;
        };
        widths[i] -= 1
    }
    let format_row = |r: &[String; 5]| -> Vec<String> {
        (0..5)
            .map(|i| {
                let s = truncate(&r[i], widths[i]);
                if i == 4 {
                    s
                } else {
                    pad(&s, widths[i])
                }
            })
            .collect()
    };
    let theme = Theme { color };
    let hs = HEADERS.map(String::from);
    let header = format_row(&hs)
        .iter()
        .map(|s| theme.header(s))
        .collect::<Vec<_>>()
        .join("  ");
    let mut out = vec![header];
    for (index, row) in rows.iter().enumerate() {
        let cells = format_row(row);
        let w = &worktrees[index];
        out.push(
            cells
                .iter()
                .enumerate()
                .map(|(col, s)| match col {
                    0 if w.current => theme.current(s),
                    1 if selected == Some(index) => theme.selected(s),
                    1 if w.branch.is_some() => theme.branch(s),
                    1 => theme.detached(s),
                    2 if selected == Some(index) => theme.wrap_selected_path(s),
                    2 => s.clone(),
                    3 => theme.commit(s),
                    4 => theme.status(w),
                    _ => s.clone(),
                })
                .collect::<Vec<_>>()
                .join("  "),
        )
    }
    out
}
