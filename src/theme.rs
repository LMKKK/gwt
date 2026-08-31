use crate::{
    status,
    types::{Availability, Worktree},
};
pub struct Theme {
    pub color: bool,
}
impl Theme {
    fn wrap(&self, code: &str, s: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.into()
        }
    }
    pub fn header(&self, s: &str) -> String {
        self.wrap("1;37", s)
    }
    pub fn current(&self, s: &str) -> String {
        self.wrap("1;32", s)
    }
    pub fn branch(&self, s: &str) -> String {
        self.wrap("36", s)
    }
    pub fn occupied_branch(&self, s: &str) -> String {
        self.wrap("33", s)
    }
    pub fn detached(&self, s: &str) -> String {
        self.wrap("33", s)
    }
    pub fn commit(&self, s: &str) -> String {
        self.wrap("2", s)
    }
    pub fn selected(&self, s: &str) -> String {
        self.wrap("1;36", s)
    }
    pub fn hint(&self, s: &str) -> String {
        self.wrap("2", s)
    }
    pub fn key(&self, s: &str) -> String {
        self.wrap("1", s)
    }
    pub fn wrap_selected_path(&self, s: &str) -> String {
        self.wrap("1", s)
    }
    pub fn status(&self, w: &Worktree) -> String {
        if !self.color {
            return status::format(w);
        }
        match w.availability {
            Availability::Unavailable | Availability::Prunable => {
                self.wrap("31", &status::format(w))
            }
            Availability::Bare => self.wrap("33", "bare"),
            Availability::Available => {
                let Some(c) = &w.status else {
                    return self.wrap("31", "unavailable");
                };
                let mut p = Vec::new();
                if c.conflicted > 0 {
                    p.push(self.wrap("31", &format!("{} conflicted", c.conflicted)))
                }
                if c.staged > 0 {
                    p.push(self.wrap("33", &format!("{} staged", c.staged)))
                }
                if c.modified > 0 {
                    p.push(self.wrap("94", &format!("{} modified", c.modified)))
                }
                if c.untracked > 0 {
                    p.push(self.wrap("31", &format!("{} untracked", c.untracked)))
                }
                if let Some(r) = &w.locked {
                    let label = if r.is_empty() {
                        "locked".into()
                    } else {
                        format!("locked: {r}")
                    };
                    p.push(self.wrap("33", &label))
                }
                if p.is_empty() {
                    self.wrap("32", "clean")
                } else {
                    p.join(", ")
                }
            }
        }
    }
}
