use crate::types::{Availability, Worktree};
pub fn format(worktree: &Worktree) -> String {
    match worktree.availability {
        Availability::Prunable => return "prunable".into(),
        Availability::Bare => return "bare".into(),
        Availability::Unavailable => return "unavailable".into(),
        Availability::Available => {}
    }
    let Some(c) = &worktree.status else {
        return "unavailable".into();
    };
    let mut p = Vec::new();
    if c.conflicted > 0 {
        p.push(format!("{} conflicted", c.conflicted));
    }
    if c.staged > 0 {
        p.push(format!("{} staged", c.staged));
    }
    if c.modified > 0 {
        p.push(format!("{} modified", c.modified));
    }
    if c.untracked > 0 {
        p.push(format!("{} untracked", c.untracked));
    }
    if let Some(reason) = &worktree.locked {
        p.push(if reason.is_empty() {
            "locked".into()
        } else {
            format!("locked: {reason}")
        });
    }
    if p.is_empty() {
        "clean".into()
    } else {
        p.join(", ")
    }
}
