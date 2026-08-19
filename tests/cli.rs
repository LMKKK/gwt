use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
fn gwt(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gwt"))
        .args(args)
        .output()
        .unwrap()
}

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gwt-cli-remove-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "gwt test")
        .env("GIT_AUTHOR_EMAIL", "gwt@example.invalid")
        .env("GIT_COMMITTER_NAME", "gwt test")
        .env("GIT_COMMITTER_EMAIL", "gwt@example.invalid")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn help_version_and_errors() {
    let v = gwt(&["--version"]);
    assert!(v.status.success());
    assert_eq!(
        String::from_utf8(v.stdout).unwrap(),
        format!("{}\n", env!("CARGO_PKG_VERSION"))
    );
    let u = gwt(&["wat"]);
    assert_eq!(u.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&u.stderr).contains("unknown command 'wat'"));
    let i = gwt(&["init", "fish"]);
    assert_eq!(i.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&i.stderr),
        "Usage: gwt init zsh|bash\n"
    );
    let help = gwt(&["--help"]);
    assert!(String::from_utf8_lossy(&help.stdout).contains("gwt remove"));
}

#[test]
fn remove_rejects_extra_arguments_and_non_tty_input() {
    let invalid = gwt(&["remove", "unexpected"]);
    assert_eq!(invalid.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&invalid.stderr),
        "Usage: gwt remove\n"
    );

    let non_tty = gwt(&["remove"]);
    assert_eq!(non_tty.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&non_tty.stderr).contains("stdin and stderr"));
}

#[test]
fn remove_cancel_exits_successfully_and_keeps_the_worktree() {
    let temp = Temp::new();
    let root = temp.0.join("main");
    let linked = temp.0.join("linked");
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-b", "main"]);
    fs::write(root.join("tracked.txt"), "initial\n").unwrap();
    git(&root, &["add", "tracked.txt"]);
    git(&root, &["commit", "-m", "initial"]);
    git(
        &root,
        &["worktree", "add", "-b", "feature", linked.to_str().unwrap()],
    );

    let binary = env!("CARGO_BIN_EXE_gwt");
    let mut command = Command::new("script");
    #[cfg(target_os = "macos")]
    command.args(["-q", "/dev/null", binary, "remove"]);
    #[cfg(target_os = "linux")]
    command.args([
        "-qec",
        &format!("'{}' remove", binary.replace('\'', "'\\''")),
        "/dev/null",
    ]);
    let mut child = command
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(b"q").unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());
    assert!(linked.is_dir());
}
#[test]
fn shell_scripts_parse() {
    for shell in ["zsh", "bash"] {
        if Command::new(shell).arg("--version").output().is_err() {
            continue;
        }
        let script = gwt(&["init", shell]).stdout;
        let mut child = Command::new(shell)
            .arg("-n")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child.stdin.take().unwrap().write_all(&script).unwrap();
        assert!(child.wait().unwrap().success())
    }
}

#[test]
fn new_requires_shell_integration_and_select_requires_a_tty() {
    let bare = gwt(&["new"]);
    assert_eq!(bare.status.code(), Some(2));
    let message = String::from_utf8_lossy(&bare.stderr);
    assert!(message.contains("gwt init zsh"));
    assert!(message.contains("gwt init bash"));

    let select = gwt(&["new", "--select"]);
    assert_eq!(select.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&select.stderr).contains("stdin and stderr"));

    let invalid = gwt(&["new", "unexpected"]);
    assert_eq!(invalid.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&invalid.stderr), "Usage: gwt new\n");
}
