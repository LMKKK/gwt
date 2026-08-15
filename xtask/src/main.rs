use serde_json::Value;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};
const PACKAGES: [(&str, &str); 6] = [
    ("aarch64-apple-darwin", "gwt-darwin-arm64"),
    ("x86_64-apple-darwin", "gwt-darwin-x64"),
    ("aarch64-unknown-linux-gnu", "gwt-linux-arm64-gnu"),
    ("aarch64-unknown-linux-musl", "gwt-linux-arm64-musl"),
    ("x86_64-unknown-linux-gnu", "gwt-linux-x64-gnu"),
    ("x86_64-unknown-linux-musl", "gwt-linux-x64-musl"),
];
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .into()
}
fn version() -> Result<String, String> {
    let text = fs::read_to_string(root().join("Cargo.toml")).map_err(|e| e.to_string())?;
    let doc = text.parse::<toml::Table>().map_err(|e| e.to_string())?;
    Ok(doc["package"]["version"].as_str().unwrap().into())
}
fn package_version(path: &Path) -> Result<String, String> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    value["version"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("missing version: {}", path.display()))
}
fn verify() -> Result<(), String> {
    let v = version()?;
    let main = root().join("packages/gwt/package.json");
    if package_version(&main)? != v {
        return Err("Cargo and npm main package versions differ".into());
    }
    let value: Value = serde_json::from_str(&fs::read_to_string(main).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    for (_, p) in PACKAGES {
        let path = root().join("packages").join(p).join("package.json");
        if package_version(&path)? != v {
            return Err(format!("version mismatch: {p}"));
        }
        let name = format!("@autumnk/{p}");
        if value["optionalDependencies"][&name].as_str() != Some(&v) {
            return Err(format!("optional dependency mismatch: {name}"));
        }
    }
    println!("all package versions are {v}");
    Ok(())
}
fn stage(target: &str) -> Result<(), String> {
    let (_, package) = PACKAGES
        .iter()
        .find(|(t, _)| *t == target)
        .ok_or_else(|| format!("unknown target: {target}"))?;
    let source = root().join("target").join(target).join("release/gwt");
    let dest = root().join("packages").join(package).join("bin/gwt");
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::copy(&source, &dest).map_err(|e| format!("{}: {e}", source.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?
    }
    println!("staged {} -> {}", source.display(), dest.display());
    Ok(())
}
fn command(mut c: Command) -> Result<(), String> {
    let display = format!("{c:?}");
    let status = c.status().map_err(|e| format!("{display}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{display} exited with {status}"))
    }
}
fn npm_pack(package: &str) -> Result<Value, String> {
    let output = Command::new("npm")
        .current_dir(root().join("packages").join(package))
        .args(["pack", "--dry-run", "--json"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    value
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| format!("npm pack returned no package for {package}"))
}
fn pack_check() -> Result<(), String> {
    command({
        let mut c = Command::new("cargo");
        c.current_dir(root()).arg("package").arg("--allow-dirty");
        c
    })?;
    for (_, p) in PACKAGES {
        let bin = root().join("packages").join(p).join("bin/gwt");
        if !bin.is_file() {
            return Err(format!("missing staged binary: {}", bin.display()));
        }
        let pack = npm_pack(p)?;
        let files = pack["files"]
            .as_array()
            .ok_or_else(|| format!("missing npm file list: {p}"))?;
        if files.len() != 2 || !files.iter().any(|f| f["path"] == "bin/gwt") {
            return Err(format!("unexpected npm contents: {p}"));
        }
    }
    let main = npm_pack("gwt")?;
    if main["files"].as_array().map_or(0, Vec::len) != 3 {
        return Err("unexpected npm main package contents".into());
    }
    Ok(())
}
fn sizes() -> Result<(), String> {
    for (t, p) in PACKAGES {
        let bin = root().join("packages").join(p).join("bin/gwt");
        if bin.exists() {
            println!(
                "{t}: {} bytes",
                fs::metadata(bin).map_err(|e| e.to_string())?.len()
            );
            let pack = npm_pack(p)?;
            println!(
                "  npm: {} compressed, {} unpacked",
                pack["size"], pack["unpackedSize"]
            );
        }
    }
    Ok(())
}
fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("verify-versions") => verify(),
        Some("stage") if args.len() == 2 => stage(&args[1]),
        Some("pack-check") => {
            verify()?;
            pack_check()
        }
        Some("sizes") => sizes(),
        _ => Err("usage: cargo xtask verify-versions|stage <target>|pack-check|sizes".into()),
    }
}
