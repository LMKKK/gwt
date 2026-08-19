use crate::{git, table, theme::Theme, types::Worktree};
use crossterm::{
    cursor::{Hide, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::io::{self, Write};
use unicode_width::UnicodeWidthStr;

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
    select_worktree(worktrees, stderr, None, "select")
        .map(|selected| selected.map(|worktree| worktree.path.clone()))
}

pub fn select_remove(worktrees: &[Worktree]) -> io::Result<Option<Worktree>> {
    select_worktree(
        worktrees,
        true,
        Some("Select a worktree to remove:"),
        "remove",
    )
    .map(|selected| selected.cloned())
}

fn select_worktree<'a>(
    worktrees: &'a [Worktree],
    stderr: bool,
    title: Option<&str>,
    action: &str,
) -> io::Result<Option<&'a Worktree>> {
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
        let table_lines = table::render(
            worktrees,
            git::terminal_width().saturating_sub(2),
            color,
            Some(selected),
        );
        let mut lines = Vec::new();
        if let Some(title) = title {
            lines.push(theme.hint(title));
        }
        lines.extend(table_lines.into_iter().enumerate().map(|(i, line)| {
            let marker = if i > 0 && i - 1 == selected {
                theme.selected("> ")
            } else {
                "  ".into()
            };
            format!("{marker}{line}")
        }));
        lines.push(format!(
            "{}{}{}{}{}{}{}{}",
            theme.key("↑/↓"),
            theme.hint(" or "),
            theme.key("j/k"),
            theme.hint(" move • "),
            theme.key("Enter"),
            theme.hint(&format!(" {action} • ")),
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
                Key::Select => return Ok(Some(&worktrees[selected])),
                Key::Up => selected = (selected + worktrees.len() - 1) % worktrees.len(),
                Key::Down => selected = (selected + 1) % worktrees.len(),
                Key::Unknown => {}
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfirmKey {
    Yes,
    No,
    Unknown,
}

pub fn parse_confirm_key(code: KeyCode, modifiers: KeyModifiers) -> ConfirmKey {
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return ConfirmKey::No;
    }
    match code {
        KeyCode::Char('y' | 'Y') => ConfirmKey::Yes,
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('n' | 'N' | 'q') => ConfirmKey::No,
        _ => ConfirmKey::Unknown,
    }
}

pub fn confirm_remove(worktree: &Worktree) -> io::Result<bool> {
    let branch = worktree.branch.as_deref().unwrap_or("detached");
    let mut output = io::stderr();
    writeln!(output, "Branch: {branch}")?;
    writeln!(output, "Path: {}", worktree.path)?;
    write!(output, "Remove this worktree? [y/N] ")?;
    output.flush()?;
    enable_raw_mode()?;
    let _guard = Guard(true);
    loop {
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            match parse_confirm_key(code, modifiers) {
                ConfirmKey::Yes => {
                    writeln!(output, "y")?;
                    return Ok(true);
                }
                ConfirmKey::No => {
                    writeln!(output)?;
                    return Ok(false);
                }
                ConfirmKey::Unknown => {}
            }
        }
    }
}

pub fn select_branch(branches: &[String]) -> io::Result<Option<String>> {
    if branches.is_empty() {
        return Ok(None);
    }
    let color = std::env::var_os("NO_COLOR").is_none();
    let theme = Theme { color };
    let mut selected = 0;
    enable_raw_mode()?;
    let _guard = Guard(true);
    let mut rendered = 0;
    loop {
        let mut output = io::stderr();
        if rendered > 0 {
            execute!(output, MoveUp(rendered as u16), MoveToColumn(0))?;
        }
        execute!(output, Hide)?;
        let mut lines = vec![theme.hint("Select a local branch:")];
        lines.extend(branches.iter().enumerate().map(|(index, branch)| {
            let marker = if index == selected {
                theme.selected("> ")
            } else {
                "  ".into()
            };
            format!("{marker}{branch}")
        }));
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
                Key::Select => return Ok(Some(branches[selected].clone())),
                Key::Up => selected = (selected + branches.len() - 1) % branches.len(),
                Key::Down => selected = (selected + 1) % branches.len(),
                Key::Unknown => {}
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathEditor {
    pub value: String,
    pub cursor: usize,
}

impl PathEditor {
    pub fn new(value: String) -> Self {
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    pub fn apply(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Option<bool> {
        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
            return Some(false);
        }
        match code {
            KeyCode::Esc => Some(false),
            KeyCode::Enter if !self.value.is_empty() => Some(true),
            KeyCode::Enter => None,
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.value.chars().count());
                None
            }
            KeyCode::Home => {
                self.cursor = 0;
                None
            }
            KeyCode::End => {
                self.cursor = self.value.chars().count();
                None
            }
            KeyCode::Backspace if self.cursor > 0 => {
                let start = char_byte_index(&self.value, self.cursor - 1);
                let end = char_byte_index(&self.value, self.cursor);
                self.value.replace_range(start..end, "");
                self.cursor -= 1;
                None
            }
            KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL) => {
                let at = char_byte_index(&self.value, self.cursor);
                self.value.insert(at, ch);
                self.cursor += 1;
                None
            }
            _ => None,
        }
    }
}

fn char_byte_index(value: &str, character: usize) -> usize {
    value
        .char_indices()
        .nth(character)
        .map_or(value.len(), |(i, _)| i)
}

pub fn input_path(default: &str) -> io::Result<Option<String>> {
    let mut editor = PathEditor::new(default.to_owned());
    enable_raw_mode()?;
    let _guard = Guard(true);
    let mut output = io::stderr();
    execute!(output, Hide)?;
    loop {
        execute!(output, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
        write!(output, "Worktree path: {}", editor.value)?;
        let cursor_byte = char_byte_index(&editor.value, editor.cursor);
        let column = "Worktree path: ".len() + editor.value[..cursor_byte].width();
        execute!(output, MoveToColumn(column.min(u16::MAX as usize) as u16))?;
        output.flush()?;
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            if let Some(submit) = editor.apply(code, modifiers) {
                write!(output, "\r\n")?;
                return Ok(submit.then_some(editor.value));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{write_lines, PathEditor};
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn raw_terminal_lines_return_to_the_left_margin() {
        let mut output = Vec::new();
        write_lines(&mut output, &["first".into(), "second".into()]).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("first\r\n"));
        assert!(text.contains("second\r\n"));
        assert!(!text.contains("first\n"));
    }

    #[test]
    fn path_editor_handles_unicode_navigation_and_cancel() {
        let mut editor = PathEditor::new("分支".into());
        editor.apply(KeyCode::Left, KeyModifiers::NONE);
        editor.apply(KeyCode::Backspace, KeyModifiers::NONE);
        editor.apply(KeyCode::Char('新'), KeyModifiers::NONE);
        assert_eq!(editor.value, "新支");
        assert_eq!(editor.apply(KeyCode::Enter, KeyModifiers::NONE), Some(true));
        assert_eq!(editor.apply(KeyCode::Esc, KeyModifiers::NONE), Some(false));
    }

    #[test]
    fn empty_path_cannot_be_submitted() {
        let mut editor = PathEditor::new(String::new());
        assert_eq!(editor.apply(KeyCode::Enter, KeyModifiers::NONE), None);
    }
}
