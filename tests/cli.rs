use std::process::Command;
fn gwt(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gwt"))
        .args(args)
        .output()
        .unwrap()
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
    )
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
