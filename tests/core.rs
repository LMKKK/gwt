use autumnk_gwt::{
    git::{parse_branches, parse_status, parse_worktrees, remove_candidates},
    status, table,
    tui::{parse_confirm_key, parse_key, ConfirmKey, Key},
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
        main: true,
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
    assert!(r[0].main);
    assert!(r[1].detached);
    assert!(!r[1].main);
    assert_eq!(r[2].prunable.as_deref(), Some("gone"));
    assert!(r[3].bare);
    assert!(!r[3].main)
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
            "    BRANCH  PATH                  COMMIT        STATUS",
            "M*  main    /repo/work tree-你好  1234567890ab  clean"
        ]
    );
    assert!(
        unicode_width::UnicodeWidthStr::width(table::truncate("worktree-你好", 8).as_str()) <= 8
    )
}

#[test]
fn renders_main_and_current_marker_combinations() {
    let mut main_current = clean();
    main_current.branch = Some("main-current".into());

    let mut main_only = clean();
    main_only.branch = Some("main-only".into());
    main_only.current = false;

    let mut current_only = clean();
    current_only.branch = Some("current-only".into());
    current_only.main = false;

    let mut linked = clean();
    linked.branch = Some("linked".into());
    linked.main = false;
    linked.current = false;

    let output = table::render(
        &[main_current, main_only, current_only, linked],
        usize::MAX,
        false,
        None,
    );
    assert!(output[1].starts_with("M*  main-current"));
    assert!(output[2].starts_with("M   main-only"));
    assert!(output[3].starts_with("*   current-only"));
    assert!(output[4].starts_with("    linked"));
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

#[test]
fn remove_candidates_exclude_unsafe_entries_but_keep_dirty_and_locked() {
    let mut main = clean();
    main.current = false;

    let mut current = clean();
    current.main = false;

    let mut bare = clean();
    bare.main = false;
    bare.current = false;
    bare.bare = true;
    bare.availability = Availability::Bare;

    let mut prunable = clean();
    prunable.main = false;
    prunable.current = false;
    prunable.prunable = Some("gone".into());
    prunable.availability = Availability::Prunable;

    let mut unavailable = clean();
    unavailable.main = false;
    unavailable.current = false;
    unavailable.availability = Availability::Unavailable;

    let mut dirty = clean();
    dirty.path = "/repo/dirty".into();
    dirty.main = false;
    dirty.current = false;
    dirty.status = Some(StatusCounts {
        modified: 1,
        ..StatusCounts::default()
    });

    let mut locked = clean();
    locked.path = "/repo/locked".into();
    locked.main = false;
    locked.current = false;
    locked.locked = Some("maintenance".into());

    let candidates = remove_candidates(&[
        main,
        current,
        bare,
        prunable,
        unavailable,
        dirty.clone(),
        locked.clone(),
    ]);
    assert_eq!(candidates, vec![dirty, locked]);
}

#[test]
fn confirmation_only_accepts_y() {
    for code in [KeyCode::Char('y'), KeyCode::Char('Y')] {
        assert_eq!(parse_confirm_key(code, KeyModifiers::NONE), ConfirmKey::Yes);
    }
    for code in [
        KeyCode::Enter,
        KeyCode::Char('n'),
        KeyCode::Char('N'),
        KeyCode::Esc,
        KeyCode::Char('q'),
    ] {
        assert_eq!(parse_confirm_key(code, KeyModifiers::NONE), ConfirmKey::No);
    }
    assert_eq!(
        parse_confirm_key(KeyCode::Char('c'), KeyModifiers::CONTROL),
        ConfirmKey::No
    );
    assert_eq!(
        parse_confirm_key(KeyCode::Char('x'), KeyModifiers::NONE),
        ConfirmKey::Unknown
    );
}
