use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::{anyhow, bail, Context};
use regex::Regex;

pub(crate) const TICK_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const TICK_CHARS: &str = "⣧⣶⣼⣹⢻⠿⡟⣏";

/// Get current username using `id -un`.
/// Unfortunately, neither `whoami` or `users` work correctly under company's
/// environment, they got nothing when trying to get current username.
/// Besides, methods using `libc::getuid` or `libc::getpwid` wrapped in an unsafe
/// block functioned uncorrectly too in company's CentOS7 server. Maybe it is
/// because there is no `passwd` table available on the server.
/// Method `libc::getlogin` does not work inside container.
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

pub struct SvnInfo {
    working_copy_root_path: String,
    url: String,
    relative_url: String,
    repo_root: String,
    repo_uuid: String,
    revision: usize,
    node_kind: String,
    schedule: String,
    last_changed_author: String,
    last_changed_revision: usize,
    last_changed_date: String,
}

impl SvnInfo {
    pub fn new() -> anyhow::Result<Self> {
        let result = Command::new("svn")
            .arg("info")
            .output()
            .context(r#"Command `svn info` failed"#)?;
        if !result.status.success() {
            bail!(anyhow!(String::from_utf8_lossy(&result.stderr).to_string())
                .context(r#"Command `svn info` failed."#));
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
        .context("Failed to build regex pattern for svn info")?;

        let captures = pattern
            .captures(&output)
            .context("Failed to capture svn info")?;

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
                .context("Can't convert revision string to number")?,
            node_kind: captures.get(7).unwrap().as_str().to_string(),
            schedule: captures.get(8).unwrap().as_str().to_string(),
            last_changed_author: captures.get(9).unwrap().as_str().to_string(),
            last_changed_revision: captures
                .get(10)
                .unwrap()
                .as_str()
                .to_string()
                .parse()
                .context("Can't convert last changed revision to number")?,
            last_changed_date: captures.get(11).unwrap().as_str().to_string(),
        })
    }

    #[allow(dead_code)]
    pub fn working_copy_root_path(&self) -> &Path {
        Path::new(&self.working_copy_root_path)
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
        let rel_url = self.relative_url();
        let branch_pattern = Regex::new(r#"\^/branches/([-[:word:]]+)"#)
            .context("Failed to build pattern for branch name")
            .unwrap();
        let branch_name = branch_pattern
            .captures(rel_url)
            .expect("Failed to match branch name")
            .get(1)
            .unwrap()
            .as_str();
        branch_name
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
    pub fn revision(&self) -> usize {
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
