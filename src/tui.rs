use crate::{
    git, table,
    theme::Theme,
    types::{BranchCandidate, BranchSelection, Worktree},
};
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

fn first_available(branches: &[BranchCandidate]) -> usize {
    branches
        .iter()
        .position(BranchCandidate::available)
        .unwrap_or(branches.len())
}

fn move_branch_selection(branches: &[BranchCandidate], selected: usize, direction: isize) -> usize {
    let mut next = selected;
    let count = branches.len() + 1;
    for _ in 0..count {
        next = (next as isize + direction).rem_euclid(count as isize) as usize;
        if next == branches.len() || branches[next].available() {
            return next;
        }
    }
    selected
}

fn render_branch_lines(
    branches: &[BranchCandidate],
    selected: Option<usize>,
    terminal_width: usize,
    color: bool,
) -> Vec<String> {
    let theme = Theme { color };
    let mut lines: Vec<String> = branches
        .iter()
        .enumerate()
        .map(|(index, branch)| {
            let marker = if selected == Some(index) {
                theme.selected("> ")
            } else {
                "  ".into()
            };
            let row = match &branch.occupied_by {
                Some(path) => format!("{}  in use: {path}", branch.name),
                None => branch.name.clone(),
            };
            let row = table::truncate(&row, terminal_width.saturating_sub(2));
            let row = if branch.available() {
                row
            } else {
                theme.hint(&row)
            };
            format!("{marker}{row}")
        })
        .collect();
    let index = branches.len();
    let marker = if selected == Some(index) {
        theme.selected("> ")
    } else {
        "  ".into()
    };
    let row = table::truncate("Create new branch...", terminal_width.saturating_sub(2));
    lines.push(format!("{marker}{}", theme.branch(&row)));
    lines
}

pub fn select_branch(branches: &[BranchCandidate]) -> io::Result<Option<BranchSelection>> {
    let color = std::env::var_os("NO_COLOR").is_none();
    let theme = Theme { color };
    let mut selected = first_available(branches);
    enable_raw_mode()?;
    let _guard = Guard(true);
    let mut rendered = 0;
    loop {
        let mut output = io::stderr();
        if rendered > 0 {
            execute!(output, MoveUp(rendered as u16), MoveToColumn(0))?;
        }
        execute!(output, Hide)?;
        let mut lines = vec![theme.hint("Select a local branch or create one:")];
        lines.extend(render_branch_lines(
            branches,
            Some(selected),
            git::terminal_width(),
            color,
        ));
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
                Key::Select => {
                    return Ok(Some(if selected == branches.len() {
                        BranchSelection::CreateNew
                    } else {
                        BranchSelection::Existing(branches[selected].clone())
                    }));
                }
                Key::Up => {
                    selected = move_branch_selection(branches, selected, -1);
                }
                Key::Down => {
                    selected = move_branch_selection(branches, selected, 1);
                }
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

fn render_editor<W: Write>(output: &mut W, prompt: &str, editor: &PathEditor) -> io::Result<()> {
    execute!(output, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
    write!(output, "{prompt}{}", editor.value)?;
    let cursor_byte = char_byte_index(&editor.value, editor.cursor);
    let column = prompt.width() + editor.value[..cursor_byte].width();
    execute!(
        output,
        MoveToColumn(column.min(u16::MAX as usize) as u16),
        Show
    )?;
    output.flush()
}

fn input_text(prompt: &str, default: &str) -> io::Result<Option<String>> {
    let mut editor = PathEditor::new(default.to_owned());
    enable_raw_mode()?;
    let _guard = Guard(true);
    let mut output = io::stderr();
    loop {
        render_editor(&mut output, prompt, &editor)?;
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

pub fn input_branch_name() -> io::Result<Option<String>> {
    input_text("New branch name: ", "")
}

pub fn input_path(default: &str) -> io::Result<Option<String>> {
    input_text("Worktree path: ", default)
}

pub fn default_worktree_path(project_name: &str, branch_name: &str) -> String {
    format!("../{project_name}_{}", branch_name.replace('/', "_"))
}

#[cfg(test)]
mod tests {
    use super::{
        default_worktree_path, first_available, move_branch_selection, render_branch_lines,
        render_editor, write_lines, PathEditor,
    };
    use crate::types::BranchCandidate;
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

    #[test]
    fn default_path_includes_project_and_sanitized_branch_name() {
        assert_eq!(default_worktree_path("gwt", "main"), "../gwt_main");
        assert_eq!(
            default_worktree_path("项目", "feature/user/profile"),
            "../项目_feature_user_profile"
        );
    }

    #[test]
    fn path_editor_rendering_shows_the_terminal_cursor() {
        let mut output = Vec::new();
        render_editor(
            &mut output,
            "Worktree path: ",
            &PathEditor::new("分支".into()),
        )
        .unwrap();
        assert!(output.windows(6).any(|bytes| bytes == b"\x1b[?25h"));
    }

    fn branch(name: &str, occupied_by: Option<&str>) -> BranchCandidate {
        BranchCandidate {
            name: name.into(),
            occupied_by: occupied_by.map(Into::into),
        }
    }

    #[test]
    fn branch_navigation_skips_occupied_entries_and_wraps() {
        let branches = [
            branch("main", Some("/repo")),
            branch("one", None),
            branch("two", Some("/repo/two")),
            branch("three", None),
        ];
        assert_eq!(first_available(&branches), 1);
        assert_eq!(move_branch_selection(&branches, 1, 1), 3);
        assert_eq!(move_branch_selection(&branches, 3, 1), 4);
        assert_eq!(move_branch_selection(&branches, 4, 1), 1);
        assert_eq!(move_branch_selection(&branches, 1, -1), 4);
    }

    #[test]
    fn all_occupied_branches_select_create_new() {
        let branches = [branch("main", Some("/repo"))];
        assert_eq!(first_available(&branches), 1);
        assert_eq!(move_branch_selection(&branches, 1, 1), 1);
    }

    #[test]
    fn occupied_branch_rendering_includes_path_and_respects_width() {
        let branches = [branch("main", Some("/repo/work tree-你好"))];
        let full = render_branch_lines(&branches, None, usize::MAX, false);
        assert_eq!(
            full,
            vec![
                "  main  in use: /repo/work tree-你好",
                "  Create new branch..."
            ]
        );
        let narrow = render_branch_lines(&branches, None, 18, false);
        assert!(unicode_width::UnicodeWidthStr::width(narrow[0].as_str()) <= 18);
        assert!(narrow[0].ends_with('…'));
        assert!(unicode_width::UnicodeWidthStr::width(narrow[1].as_str()) <= 18);
    }

    #[test]
    fn empty_branch_list_still_offers_create_new() {
        let branches = [];
        assert_eq!(first_available(&branches), 0);
        assert_eq!(
            render_branch_lines(&branches, Some(0), usize::MAX, false),
            vec!["> Create new branch..."]
        );
    }
}
