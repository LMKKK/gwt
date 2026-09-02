use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
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

fn wait_for_output(receiver: &Receiver<Vec<u8>>, output: &mut Vec<u8>, expected: &[u8]) {
    while !output
        .windows(expected.len())
        .any(|window| window == expected)
    {
        let chunk = receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or_else(|_| {
                panic!(
                    "timed out waiting for {:?} in {:?}",
                    String::from_utf8_lossy(expected),
                    String::from_utf8_lossy(output)
                )
            });
        output.extend_from_slice(&chunk);
    }
}

fn run_new_in_shell(
    cwd: &Path,
    branch: &str,
    confirmation: &[u8],
) -> (std::process::ExitStatus, Vec<u8>) {
    let binary = env!("CARGO_BIN_EXE_gwt");
    let binary_dir = Path::new(binary).parent().unwrap();
    let shell_command = format!(
        "stty cols 120 rows 40; PATH='{}':\"$PATH\"; eval \"$('{binary}' init bash)\"; gwt new; printf '\\nPWD_AFTER:%s\\n' \"$PWD\"",
        binary_dir.display()
    );
    let mut command = Command::new("script");
    #[cfg(target_os = "macos")]
    command.args(["-q", "/dev/null", "bash", "-c", &shell_command]);
    #[cfg(target_os = "linux")]
    command.args([
        "-qec",
        &format!("bash -c '{}'", shell_command.replace('\'', "'\\''")),
        "/dev/null",
    ]);
    let mut child = command
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut terminal = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            let read = terminal.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            sender.send(buffer[..read].to_vec()).unwrap();
        }
    });
    let mut output = Vec::new();

    wait_for_output(
        &receiver,
        &mut output,
        b"Select a local branch or create one:",
    );
    input.write_all(b"\r").unwrap();
    wait_for_output(&receiver, &mut output, b"New branch name: ");
    input.write_all(branch.as_bytes()).unwrap();
    input.write_all(b"\r").unwrap();
    wait_for_output(&receiver, &mut output, b"Worktree path: ");
    input.write_all(b"\r").unwrap();
    wait_for_output(
        &receiver,
        &mut output,
        b"Switch to the new worktree? [y/N] ",
    );
    input.write_all(confirmation).unwrap();
    drop(input);

    let status = child.wait().unwrap();
    reader.join().unwrap();
    output.extend(receiver.into_iter().flatten());
    (status, output)
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

#[test]
fn new_asks_before_switching_but_always_creates_the_worktree() {
    if Command::new("bash").arg("--version").output().is_err() {
        return;
    }

    let temp = Temp::new();
    let root = temp.0.join("main");
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-b", "main"]);
    fs::write(root.join("tracked.txt"), "initial\n").unwrap();
    git(&root, &["add", "tracked.txt"]);
    git(&root, &["commit", "-m", "initial"]);

    let (stayed_status, stayed_output) = run_new_in_shell(&root, "feature/no-switch", b"\r");
    assert!(stayed_status.success());
    let stayed_path = temp.0.join("main_feature_no-switch");
    assert!(stayed_path.is_dir());
    let canonical_root = fs::canonicalize(&root).unwrap();
    assert!(String::from_utf8_lossy(&stayed_output)
        .contains(&format!("PWD_AFTER:{}", canonical_root.display())));

    let (switched_status, switched_output) = run_new_in_shell(&root, "feature/switch", b"y");
    assert!(switched_status.success());
    let switched_path = temp.0.join("main_feature_switch");
    assert!(switched_path.is_dir());
    let canonical_switched_path = fs::canonicalize(&switched_path).unwrap();
    assert!(String::from_utf8_lossy(&switched_output)
        .contains(&format!("PWD_AFTER:{}", canonical_switched_path.display())));
}
