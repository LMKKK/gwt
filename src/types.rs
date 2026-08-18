#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    Prunable,
    Bare,
    Unavailable,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusCounts {
    pub conflicted: usize,
    pub staged: usize,
    pub modified: usize,
    pub untracked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    pub path: String,
    pub head: String,
    pub branch: Option<String>,
    pub detached: bool,
    pub bare: bool,
    pub locked: Option<String>,
    pub prunable: Option<String>,
    pub main: bool,
    pub current: bool,
    pub availability: Availability,
    pub status: Option<StatusCounts>,
}

impl Worktree {
    pub fn record(path: String) -> Self {
        Self {
            path,
            head: String::new(),
            branch: None,
            detached: false,
            bare: false,
            locked: None,
            prunable: None,
            main: false,
            current: false,
            availability: Availability::Unavailable,
            status: None,
        }
    }
}
