use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::{ffi::OsString, fmt::Display};

use anyhow::{Context, anyhow, bail};
use regex::Regex;

pub(crate) mod progress_bar {
    use std::time::Duration;

    pub(crate) const TICK_INTERVAL: Duration = Duration::from_millis(120);
    pub(crate) const TICK_CHARS: &str = "⣧⣶⣼⣹⢻⠿⡟⣏";
}

#[allow(unused)]
pub(crate) mod symbols {
    pub(crate) const LINE_H: &str = "─";
    pub(crate) const LINE_H_HEAVY: &str = "━";
    pub(crate) const LINE_HD: &str = "═";
    pub(crate) const LINE_V: &str = "│";
    pub(crate) const LINE_VD: &str = "║";
    pub(crate) const SQUARE_FULL: &str = "█";
    pub(crate) const DIAMOND: &str = "◆";
}

/// Get current username by `id -un`. Unfortunately, neither `whoami` or `users` work correctly
/// under company's dev environment. Besides, methods by wrapping `libc::getuid` or `libc::getpwid`
/// or `libc::getlogin` does not work too on the CentOS7 server in company. Maybe there is no
/// `passwd` table available.
#[allow(dead_code)]
pub fn get_current_username() -> Option<String> {
    let output = Command::new("id").arg("-un").output();

    if output.is_err() {
        return None;
    }

    let output = output.unwrap();
    if !output.status.success() {
        return None;
    }

    Some(
        OsString::from_vec(output.stdout)
            .into_string()
            .unwrap()
            .trim()
            .to_string(),
    )
}

#[derive(Clone, Debug)]
pub struct SvnInfo {
    working_copy_root_path: String,
    url: String,
    relative_url: String,
    repo_root: String,
    repo_uuid: String,
    revision: i64,
    node_kind: String,
    schedule: String,
    last_changed_author: String,
    last_changed_revision: usize,
    last_changed_date: String,
}

impl Display for SvnInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"SvnInfo {{
working_copy_root_path: {},
url: {},
relative_url: {},
repo_root: {},
repo_uuid: {},
revision: {},
node_kind: {},
schedule: {},
last_changed_author: {},
last_changed_revision: {},
last_changed_date: {}
}}"#,
            self.working_copy_root_path,
            self.url,
            self.relative_url,
            self.repo_root,
            self.repo_uuid,
            self.revision,
            self.node_kind,
            self.schedule,
            self.last_changed_author,
            self.last_changed_revision,
            self.last_changed_date
        )
    }
}

impl SvnInfo {
    pub fn new() -> anyhow::Result<Self> {
        let result = Command::new("svn")
            .arg("info")
            .output()
            .context(r#"Command `svn info` failed"#)?;
        if !result.status.success() {
            bail!(
                anyhow!(String::from_utf8_lossy(&result.stderr).to_string())
                    .context(r#"Command `svn info` failed."#)
            );
        }

        let output = String::from_utf8_lossy(&result.stdout).to_string();
        let pattern = Regex::new(
            r#"Working Copy Root Path: ([^\n]+)
URL: ([^\n]+)
Relative URL: ([^\n]+)
Repository Root: ([^\n]+)
Repository UUID: ([^\n]+)
Revision: ([^\n]+)
Node Kind: ([^\n]+)
Schedule: ([^\n]+)
Last Changed Author: ([^\n]+)
Last Changed Rev: ([[:digit:]]+)
Last Changed Date: ([^\n]+)"#,
        )
        .context("Failed to construct regex for svn info")?;

        let captures = pattern
            .captures(&output)
            .context("Capture svn info failed")?;

        Ok(SvnInfo {
            working_copy_root_path: captures.get(1).unwrap().as_str().to_string(),
            url: captures.get(2).unwrap().as_str().to_string(),
            relative_url: captures.get(3).unwrap().as_str().to_string(),
            repo_root: captures.get(4).unwrap().as_str().to_string(),
            repo_uuid: captures.get(5).unwrap().as_str().to_string(),
            revision: captures
                .get(6)
                .unwrap()
                .as_str()
                .to_string()
                .parse()
                .context("Parse revision failed")?,
            node_kind: captures.get(7).unwrap().as_str().to_string(),
            schedule: captures.get(8).unwrap().as_str().to_string(),
            last_changed_author: captures.get(9).unwrap().as_str().to_string(),
            last_changed_revision: captures
                .get(10)
                .unwrap()
                .as_str()
                .to_string()
                .parse()
                .context("Parse last changed revision failed")?,
            last_changed_date: captures.get(11).unwrap().as_str().to_string(),
        })
    }

    #[allow(dead_code)]
    pub fn working_copy_root_path(&self) -> &Path {
        Path::new(self.working_copy_root_path.as_str())
    }

    #[allow(dead_code)]
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    #[allow(dead_code)]
    pub fn relative_url(&self) -> &str {
        self.relative_url.as_str()
    }

    #[allow(dead_code)]
    pub fn branch_name(&self) -> &str {
        static REGEX_BRANCH: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"\^/branches/([^/]+)"#).unwrap());

        REGEX_BRANCH
            .captures(self.relative_url.as_str())
            .expect("Capture branch failed")
            .get(1)
            .unwrap()
            .as_str()
    }

    #[allow(dead_code)]
    pub fn repository_root(&self) -> &str {
        self.repo_root.as_str()
    }

    #[allow(dead_code)]
    pub fn repository_uuid(&self) -> &str {
        self.repo_uuid.as_str()
    }

    #[allow(dead_code)]
    pub fn revision(&self) -> i64 {
        self.revision
    }

    #[allow(dead_code)]
    pub fn node_kind(&self) -> &str {
        self.node_kind.as_str()
    }

    #[allow(dead_code)]
    pub fn schedule(&self) -> &str {
        self.schedule.as_str()
    }

    #[allow(dead_code)]
    pub fn last_changed_author(&self) -> &str {
        self.last_changed_author.as_str()
    }

    #[allow(dead_code)]
    pub fn last_changed_revision(&self) -> usize {
        self.last_changed_revision
    }

    #[allow(dead_code)]
    pub fn last_changed_date(&self) -> &str {
        self.last_changed_date.as_str()
    }
}

pub(crate) fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            std::path::Component::RootDir => {
                normalized.push(component);
            }
            std::path::Component::ParentDir => {
                normalized.pop(); // Go up one directory
            }
            std::path::Component::Normal(v) => {
                normalized.push(v); // Push normal components
            }
            _ => {} // Skip current directory (.) and others
        }
    }
    normalized
}
