use autumnk_gwt::{
    git::{parse_branches, parse_status, parse_worktrees},
    status, table,
    tui::{parse_key, Key},
    types::{Availability, StatusCounts, Worktree},
};
use crossterm::event::{KeyCode, KeyModifiers};
fn clean() -> Worktree {
    Worktree {
        path: "/repo/work tree-你好".into(),
        head: "1234567890abcdef".into(),
        branch: Some("main".into()),
        detached: false,
        bare: false,
        locked: None,
        prunable: None,
        current: true,
        availability: Availability::Available,
        status: Some(StatusCounts::default()),
    }
}
#[test]
fn parses_worktree_porcelain() {
    let input = [
        "worktree /repo",
        "HEAD abcdef",
        "branch refs/heads/main",
        "locked maintenance",
        "",
        "worktree /repo/detached",
        "HEAD 123456",
        "detached",
        "",
        "worktree /repo/missing",
        "HEAD fedcba",
        "prunable gone",
        "",
        "worktree /repo.git",
        "HEAD 000000",
        "bare",
        "",
        "",
    ]
    .join("\0");
    let r = parse_worktrees(input.as_bytes());
    assert_eq!(r.len(), 4);
    assert_eq!(r[0].branch.as_deref(), Some("main"));
    assert_eq!(r[0].locked.as_deref(), Some("maintenance"));
    assert!(r[1].detached);
    assert_eq!(r[2].prunable.as_deref(), Some("gone"));
    assert!(r[3].bare)
}

#[test]
fn parses_local_branch_names() {
    assert_eq!(
        parse_branches(b"feature/one\nmain\nrelease-\xe4\xbd\xa0\xe5\xa5\xbd\n"),
        vec!["feature/one", "main", "release-你好"]
    );
}
#[test]
fn parses_status_dimensions_and_rename() {
    let input = [
        "1 M. N... x",
        "1 .M N... x",
        "1 MM N... x",
        "? x",
        "u UU N... x",
        "2 R. N... x",
        "old",
        "? next",
        "",
    ]
    .join("\0");
    assert_eq!(
        parse_status(input.as_bytes()),
        StatusCounts {
            conflicted: 1,
            staged: 3,
            modified: 2,
            untracked: 2
        }
    )
}
#[test]
fn status_and_table_are_compatible() {
    let w = clean();
    assert_eq!(status::format(&w), "clean");
    assert_eq!(
        table::render(&[w], usize::MAX, false, None),
        vec![
            "   BRANCH  PATH                  COMMIT        STATUS",
            "*  main    /repo/work tree-你好  1234567890ab  clean"
        ]
    );
    assert!(
        unicode_width::UnicodeWidthStr::width(table::truncate("worktree-你好", 8).as_str()) <= 8
    )
}
#[test]
fn keys_match() {
    assert_eq!(parse_key(KeyCode::Char('j'), KeyModifiers::NONE), Key::Down);
    assert_eq!(parse_key(KeyCode::Up, KeyModifiers::NONE), Key::Up);
    assert_eq!(parse_key(KeyCode::Enter, KeyModifiers::NONE), Key::Select);
    assert_eq!(
        parse_key(KeyCode::Char('c'), KeyModifiers::CONTROL),
        Key::Cancel
    )
}
