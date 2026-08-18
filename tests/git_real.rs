use autumnk_gwt::{
    git,
    types::{Availability, StatusCounts},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
struct Temp(PathBuf);
impl Temp {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "gwt-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn run(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "gwt test")
        .env("GIT_AUTHOR_EMAIL", "gwt@example.invalid")
        .env("GIT_COMMITTER_NAME", "gwt test")
        .env("GIT_COMMITTER_EMAIL", "gwt@example.invalid")
        .output()
        .unwrap()
}
fn ok(cwd: &Path, args: &[&str]) {
    let o = run(cwd, args);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr))
}
#[test]
fn lists_subdirectory_spaces_unicode_and_status() {
    let t = Temp::new("real");
    let root = t.0.join("main");
    fs::create_dir(&root).unwrap();
    ok(&root, &["init", "-b", "main"]);
    fs::write(root.join("tracked.txt"), "initial\n").unwrap();
    ok(&root, &["add", "tracked.txt"]);
    ok(&root, &["commit", "-m", "initial"]);
    let linked = t.0.join("work tree-你好");
    ok(
        &root,
        &["worktree", "add", "-b", "feature", linked.to_str().unwrap()],
    );
    fs::write(linked.join("tracked.txt"), "changed\n").unwrap();
    fs::write(linked.join("new file.txt"), "new\n").unwrap();
    ok(&linked, &["add", "new file.txt"]);
    fs::write(linked.join("untracked.txt"), "new\n").unwrap();
    let nested = linked.join("nested");
    fs::create_dir(&nested).unwrap();
    let list = git::list(&nested).unwrap();
    assert_eq!(list.len(), 2);
    let main = list.iter().find(|w| w.main).unwrap();
    assert_eq!(main.branch.as_deref(), Some("main"));
    assert!(!main.current);
    let current = list.iter().find(|w| w.current).unwrap();
    assert_eq!(current.branch.as_deref(), Some("feature"));
    assert!(!current.main);
    assert_eq!(
        current.status,
        Some(StatusCounts {
            conflicted: 0,
            staged: 1,
            modified: 1,
            untracked: 1
        })
    );
    let rendered = autumnk_gwt::table::render(&list, usize::MAX, false, None);
    assert!(
        rendered.iter().any(|line| line.starts_with("M   main")),
        "{rendered:#?}"
    );
    assert!(
        rendered.iter().any(|line| line.starts_with("*   feature")),
        "{rendered:#?}"
    )
}
#[test]
fn reports_bare_repo() {
    let t = Temp::new("bare");
    ok(&t.0, &["init", "--bare"]);
    let list = git::list(&t.0).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].availability, Availability::Bare);
    assert!(!list[0].main)
}
#[test]
fn reports_not_a_repository() {
    let t = Temp::new("none");
    assert!(git::list(&t.0)
        .unwrap_err()
        .to_string()
        .starts_with("Not a Git repository:"))
}

#[test]
fn filters_occupied_branches_and_creates_relative_to_cwd() {
    let t = Temp::new("new");
    let root = t.0.join("main");
    fs::create_dir(&root).unwrap();
    ok(&root, &["init", "-b", "main"]);
    fs::write(root.join("tracked.txt"), "initial\n").unwrap();
    ok(&root, &["add", "tracked.txt"]);
    ok(&root, &["commit", "-m", "initial"]);
    ok(&root, &["branch", "feature/one"]);
    ok(&root, &["branch", "unicode-你好"]);

    assert_eq!(
        git::available_branches(&root).unwrap(),
        vec!["feature/one", "unicode-你好"]
    );

    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    let destination = nested.join("work tree-你好");
    git::add_worktree(&nested, &destination, "unicode-你好").unwrap();
    assert!(destination.is_dir());
    let branch = run(&destination, &["branch", "--show-current"]);
    assert_eq!(
        String::from_utf8_lossy(&branch.stdout).trim(),
        "unicode-你好"
    );
    assert_eq!(git::available_branches(&root).unwrap(), vec!["feature/one"]);
}

#[test]
fn creating_at_an_existing_path_preserves_git_error() {
    let t = Temp::new("new-conflict");
    ok(&t.0, &["init", "-b", "main"]);
    fs::write(t.0.join("tracked.txt"), "initial\n").unwrap();
    ok(&t.0, &["add", "tracked.txt"]);
    ok(&t.0, &["commit", "-m", "initial"]);
    ok(&t.0, &["branch", "feature"]);
    let occupied = t.0.join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("file"), "conflict").unwrap();
    let error = git::add_worktree(&t.0, &occupied, "feature").unwrap_err();
    assert!(error.to_string().contains("already exists"));
}

#[test]
fn counts_a_real_merge_conflict() {
    let t = Temp::new("conflict");
    ok(&t.0, &["init", "-b", "main"]);
    fs::write(t.0.join("conflict.txt"), "base\n").unwrap();
    ok(&t.0, &["add", "conflict.txt"]);
    ok(&t.0, &["commit", "-m", "base"]);
    ok(&t.0, &["switch", "-c", "other"]);
    fs::write(t.0.join("conflict.txt"), "other\n").unwrap();
    ok(&t.0, &["commit", "-am", "other"]);
    ok(&t.0, &["switch", "main"]);
    fs::write(t.0.join("conflict.txt"), "main\n").unwrap();
    ok(&t.0, &["commit", "-am", "main"]);
    assert!(!run(&t.0, &["merge", "other"]).status.success());
    assert_eq!(
        git::list(&t.0).unwrap()[0].status,
        Some(StatusCounts {
            conflicted: 1,
            staged: 0,
            modified: 0,
            untracked: 0
        })
    );
}
