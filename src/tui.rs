use crate::{git, table, theme::Theme, types::Worktree};
use crossterm::{
    cursor::{Hide, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::io::{self, Write};

fn write_lines<W: Write>(output: &mut W, lines: &[String]) -> io::Result<()> {
    for line in lines {
        execute!(output, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
        write!(output, "{line}\r\n")?
    }
    Ok(())
}
#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Select,
    Cancel,
    Unknown,
}
pub fn parse_key(code: KeyCode, mods: KeyModifiers) -> Key {
    if mods.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return Key::Cancel;
    }
    match code {
        KeyCode::Enter => Key::Select,
        KeyCode::Down | KeyCode::Char('j') => Key::Down,
        KeyCode::Up | KeyCode::Char('k') => Key::Up,
        KeyCode::Esc | KeyCode::Char('q') => Key::Cancel,
        _ => Key::Unknown,
    }
}
struct Guard(bool);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        if self.0 {
            let _ = execute!(io::stderr(), Show);
        } else {
            let _ = execute!(io::stdout(), Show);
        }
    }
}
pub fn select(worktrees: &[Worktree], stderr: bool) -> io::Result<Option<String>> {
    if worktrees.is_empty() {
        return Ok(None);
    }
    let color = std::env::var_os("NO_COLOR").is_none();
    let mut selected = worktrees.iter().position(|w| w.current).unwrap_or(0);
    enable_raw_mode()?;
    let _guard = Guard(stderr);
    let mut rendered = 0;
    loop {
        let mut output: Box<dyn Write> = if stderr {
            Box::new(io::stderr())
        } else {
            Box::new(io::stdout())
        };
        if rendered > 0 {
            execute!(output, MoveUp(rendered as u16), MoveToColumn(0))?
        }
        execute!(output, Hide)?;
        let theme = Theme { color };
        let mut lines = table::render(
            worktrees,
            git::terminal_width().saturating_sub(2),
            color,
            Some(selected),
        );
        for (i, line) in lines.iter_mut().enumerate() {
            let marker = if i > 0 && i - 1 == selected {
                theme.selected("> ")
            } else {
                "  ".into()
            };
            *line = format!("{marker}{line}")
        }
        lines.push(format!(
            "{}{}{}{}{}{}{}{}",
            theme.key("↑/↓"),
            theme.hint(" or "),
            theme.key("j/k"),
            theme.hint(" move • "),
            theme.key("Enter"),
            theme.hint(" select • "),
            theme.key("q"),
            theme.hint(" cancel")
        ));
        write_lines(&mut output, &lines)?;
        output.flush()?;
        rendered = lines.len();
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            match parse_key(code, modifiers) {
                Key::Cancel => return Ok(None),
                Key::Select => return Ok(Some(worktrees[selected].path.clone())),
                Key::Up => selected = (selected + worktrees.len() - 1) % worktrees.len(),
                Key::Down => selected = (selected + 1) % worktrees.len(),
                Key::Unknown => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::write_lines;

    #[test]
    fn raw_terminal_lines_return_to_the_left_margin() {
        let mut output = Vec::new();
        write_lines(&mut output, &["first".into(), "second".into()]).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("first\r\n"));
        assert!(text.contains("second\r\n"));
        assert!(!text.contains("first\n"));
    }
}
