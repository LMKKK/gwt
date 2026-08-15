use crate::{git, shell, table, tui};
use std::io::{self, IsTerminal, Write};
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
fn help() -> String {
    format!("gwt {VERSION} — switch between Git worktrees\n\nUsage:\n  gwt list          List and switch between worktrees\n  gwt new           Create and switch to a worktree (shell integration required)\n  gwt init zsh      Print zsh integration\n  gwt init bash     Print bash integration\n  gwt --help        Show help\n  gwt --version     Show version\n\nInstall shell integration so list and new can change the current directory:\n  eval \"$(gwt init zsh)\"\n  eval \"$(gwt init bash)\"")
}
pub fn run(args: Vec<String>) -> u8 {
    match run_inner(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("gwt: {e}");
            1
        }
    }
}
fn run_inner(args: &[String]) -> Result<u8, Box<dyn std::error::Error>> {
    let first = args.first().map(String::as_str);
    match first {
        None | Some("--help" | "-h" | "help") => {
            println!("{}", help());
            Ok(0)
        }
        Some("--version" | "-v") => {
            println!("{VERSION}");
            Ok(0)
        }
        Some("init") => {
            if args.len() != 2 || !matches!(args[1].as_str(), "zsh" | "bash") {
                eprintln!("Usage: gwt init zsh|bash");
                return Ok(2);
            }
            println!("{}", shell::init());
            Ok(0)
        }
        Some("list") => {
            let select = args.len() == 2 && args[1] == "--select";
            if (!select && args.len() != 1) || args.len() > 2 {
                eprintln!("Usage: gwt list");
                return Ok(2);
            }
            let stdin_tty = io::stdin().is_terminal();
            let output_tty = if select {
                io::stderr().is_terminal()
            } else {
                io::stdout().is_terminal()
            };
            if select && !(stdin_tty && output_tty) {
                eprintln!("gwt list --select requires an interactive terminal.");
                return Ok(2);
            }
            let worktrees = git::list(&git::cwd()?)?;
            if stdin_tty && output_tty {
                if let Some(path) = tui::select(&worktrees, select)? {
                    if select {
                        println!("{path}")
                    }
                }
            } else {
                let stdout = io::stdout();
                let mut lock = stdout.lock();
                for line in table::render(&worktrees, usize::MAX, false, None) {
                    writeln!(lock, "{line}")?
                }
            }
            Ok(0)
        }
        Some("new") => {
            if args.len() == 1 {
                eprintln!("gwt new requires shell integration. Run 'eval \"$(gwt init zsh)\"' or 'eval \"$(gwt init bash)\"' first.");
                return Ok(2);
            }
            if args.len() != 2 || args[1] != "--select" {
                eprintln!("Usage: gwt new");
                return Ok(2);
            }
            if !(io::stdin().is_terminal() && io::stderr().is_terminal()) {
                eprintln!("gwt new --select requires an interactive terminal on stdin and stderr.");
                return Ok(2);
            }
            let cwd = git::cwd()?;
            let branches = git::available_branches(&cwd)?;
            if branches.is_empty() {
                return Err("No available local branches. Every local branch is already checked out in a worktree.".into());
            }
            let Some(branch) = tui::select_branch(&branches)? else {
                return Ok(0);
            };
            let Some(input) = tui::input_path(&branch)? else {
                return Ok(0);
            };
            let path = std::path::Path::new(&input);
            let absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            };
            git::add_worktree(&cwd, &absolute, &branch)?;
            println!("{}", absolute.display());
            Ok(0)
        }
        Some(other) => {
            eprintln!("gwt: unknown command '{other}'. Run 'gwt --help' for usage.");
            Ok(2)
        }
    }
}
