use crate::types::{Availability, BranchCandidate, StatusCounts, Worktree};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    thread,
};

#[derive(Debug)]
pub struct GitError(pub String);
impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for GitError {}

trait GitRunner: Send + Sync {
    fn run(&self, args: &[&str], cwd: &Path) -> Result<Vec<u8>, GitError>;
}
struct SystemGit;
impl GitRunner for SystemGit {
    fn run(&self, args: &[&str], cwd: &Path) -> Result<Vec<u8>, GitError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|_| {
                GitError(
                    "Unable to run Git. Make sure 'git' is installed and available in PATH.".into(),
                )
            })?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if detail.contains("not a git repository") {
            Err(GitError(format!("Not a Git repository: {}", cwd.display())))
        } else {
            Err(GitError(format!(
                "Git command failed: git {}{}",
                args.join(" "),
                if detail.is_empty() {
                    String::new()
                } else {
                    format!("\n{detail}")
                }
            )))
        }
    }
}

pub fn parse_worktrees(input: &[u8]) -> Vec<Worktree> {
    let mut records = Vec::new();
    let mut current: Option<Worktree> = None;
    for raw in input.split(|b| *b == 0) {
        if raw.is_empty() {
            if let Some(item) = current.take() {
                records.push(item);
            }
            continue;
        }
        let field = String::from_utf8_lossy(raw);
        let (key, value) = field.split_once(' ').unwrap_or((&field, ""));
        if key == "worktree" {
            if let Some(item) = current.take() {
                records.push(item);
            }
            current = Some(Worktree::record(value.into()));
            continue;
        }
        let Some(item) = current.as_mut() else {
            continue;
        };
        match key {
            "HEAD" => item.head = value.into(),
            "branch" => {
                item.branch = Some(value.strip_prefix("refs/heads/").unwrap_or(value).into())
            }
            "detached" => item.detached = true,
            "bare" => item.bare = true,
            "locked" => item.locked = Some(value.into()),
            "prunable" => item.prunable = Some(value.into()),
            _ => {}
        }
    }
    if let Some(item) = current {
        records.push(item);
    }
    if let Some(item) = records.iter_mut().find(|item| !item.bare) {
        item.main = true;
    }
    records
}

pub fn parse_branches(input: &[u8]) -> Vec<String> {
    input
        .split(|byte| *byte == b'\n')
        .filter(|name| !name.is_empty())
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .collect()
}

pub fn parse_status(input: &[u8]) -> StatusCounts {
    let entries: Vec<_> = input.split(|b| *b == 0).collect();
    let mut result = StatusCounts::default();
    let mut i = 0;
    while i < entries.len() {
        let entry = entries[i];
        if entry.starts_with(b"? ") {
            result.untracked += 1;
        } else if entry.starts_with(b"u ") {
            result.conflicted += 1;
        } else if entry.starts_with(b"1 ") || entry.starts_with(b"2 ") {
            if entry.get(2) != Some(&b'.') {
                result.staged += 1;
            }
            if entry.get(3) != Some(&b'.') {
                result.modified += 1;
            }
            if entry.starts_with(b"2 ") {
                i += 1;
            }
        }
        i += 1;
    }
    result
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
fn inside(candidate: &Path, root: &Path) -> bool {
    canonical(candidate).starts_with(canonical(root))
}

pub fn list(cwd: &Path) -> Result<Vec<Worktree>, GitError> {
    list_with(&SystemGit, cwd)
}

pub fn available_branches(cwd: &Path) -> Result<Vec<String>, GitError> {
    Ok(branch_candidates(cwd)?
        .into_iter()
        .filter(BranchCandidate::available)
        .map(|branch| branch.name)
        .collect())
}

pub fn branch_candidates(cwd: &Path) -> Result<Vec<BranchCandidate>, GitError> {
    let runner = SystemGit;
    let branches = parse_branches(&runner.run(
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
        cwd,
    )?);
    let occupied: HashMap<String, String> =
        parse_worktrees(&runner.run(&["worktree", "list", "--porcelain", "-z"], cwd)?)
            .into_iter()
            .filter_map(|worktree| worktree.branch.map(|branch| (branch, worktree.path)))
            .collect();
    Ok(branches
        .into_iter()
        .map(|name| BranchCandidate {
            occupied_by: occupied.get(&name).cloned(),
            name,
        })
        .collect())
}

pub fn main_worktree_name(cwd: &Path) -> Result<String, GitError> {
    let runner = SystemGit;
    let worktrees = parse_worktrees(&runner.run(&["worktree", "list", "--porcelain", "-z"], cwd)?);
    let main = worktrees
        .iter()
        .find(|worktree| worktree.main)
        .ok_or_else(|| GitError("Unable to determine the main worktree.".into()))?;
    Path::new(&main.path)
        .file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| GitError("Unable to determine the main worktree name.".into()))
}

pub fn remove_candidates(worktrees: &[Worktree]) -> Vec<Worktree> {
    worktrees
        .iter()
        .filter(|worktree| {
            !worktree.main
                && !worktree.current
                && !worktree.bare
                && worktree.prunable.is_none()
                && worktree.availability == Availability::Available
        })
        .cloned()
        .collect()
}

pub fn add_worktree(cwd: &Path, path: &Path, branch: &str) -> Result<(), GitError> {
    let output = Command::new("git")
        .args(["worktree", "add"])
        .arg(path)
        .arg(branch)
        .current_dir(cwd)
        .output()
        .map_err(|_| {
            GitError(
                "Unable to run Git. Make sure 'git' is installed and available in PATH.".into(),
            )
        })?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(GitError(if detail.is_empty() {
        format!(
            "Git command failed: git worktree add {} {branch}",
            path.display()
        )
    } else {
        format!(
            "Git command failed: git worktree add {} {branch}\n{detail}",
            path.display()
        )
    }))
}

pub fn remove_worktree(cwd: &Path, path: &Path) -> Result<(), GitError> {
    let output = Command::new("git")
        .args(["worktree", "remove"])
        .arg(path)
        .current_dir(cwd)
        .output()
        .map_err(|_| {
            GitError(
                "Unable to run Git. Make sure 'git' is installed and available in PATH.".into(),
            )
        })?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(GitError(if detail.is_empty() {
        format!("Git command failed: git worktree remove {}", path.display())
    } else {
        format!(
            "Git command failed: git worktree remove {}\n{detail}",
            path.display()
        )
    }))
}
fn list_with(runner: &dyn GitRunner, cwd: &Path) -> Result<Vec<Worktree>, GitError> {
    let mut records =
        parse_worktrees(&runner.run(&["worktree", "list", "--porcelain", "-z"], cwd)?);
    let current = records
        .iter()
        .enumerate()
        .filter(|(_, r)| inside(cwd, Path::new(&r.path)))
        .max_by_key(|(_, r)| r.path.len())
        .map(|(i, _)| i);
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for (index, mut record) in records.drain(..).enumerate() {
            handles.push(scope.spawn(move || {
                record.current = current == Some(index);
                record.availability = if record.prunable.is_some() {
                    Availability::Prunable
                } else if record.bare {
                    Availability::Bare
                } else if !Path::new(&record.path).exists() {
                    Availability::Unavailable
                } else {
                    Availability::Available
                };
                if record.availability == Availability::Available {
                    match runner.run(
                        &["status", "--porcelain=v2", "-z", "--untracked-files=all"],
                        Path::new(&record.path),
                    ) {
                        Ok(out) => record.status = Some(parse_status(&out)),
                        Err(_) => record.availability = Availability::Unavailable,
                    }
                }
                record
            }));
        }
        records = handles
            .into_iter()
            .map(|h| h.join().expect("status worker panicked"))
            .collect();
    });
    Ok(records)
}

pub fn terminal_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| usize::from(w))
        .unwrap_or(120)
}
pub fn cwd() -> io::Result<PathBuf> {
    std::env::current_dir()
}
