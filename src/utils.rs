use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Context};
use regex::Regex;

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
    /// Return an empty template of SvnInfo
    pub fn new() -> anyhow::Result<Self> {
        let mut instance = SvnInfo {
            working_copy_root_path: String::new(),
            url: String::new(),
            relative_url: String::new(),
            repo_root: String::new(),
            repo_uuid: String::new(),
            revision: 0,
            node_kind: String::new(),
            schedule: String::new(),
            last_changed_author: String::new(),
            last_changed_revision: 0,
            last_changed_date: String::new(),
        };

        instance.info()?;
        Ok(instance)
    }

    /// Clear the contents inside
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.working_copy_root_path.clear();
        self.url.clear();
        self.relative_url.clear();
        self.repo_root.clear();
        self.repo_uuid.clear();
        self.revision = 0;
        self.node_kind.clear();
        self.schedule.clear();
        self.last_changed_author.clear();
        self.last_changed_revision = 0;
        self.last_changed_date.clear();
    }

    /// Invoking .info method means executing `svn info` command once and storing its output
    #[allow(dead_code)]
    pub fn info(&mut self) -> anyhow::Result<()> {
        let output = Command::new("svn")
            .arg("info")
            .output()
            .context(r#"Command `svn info` failed"#)?;
        if !output.status.success() {
            bail!(anyhow!(String::from_utf8_lossy(&output.stderr).to_string())
                .context(r#"Command `svn info` failed."#));
        }

        let info = String::from_utf8_lossy(&output.stdout).to_string();
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
        .context("Failed to build pattern for capturing project root dir")?;

        let captures = pattern.captures(&info).context("Failed to capture svn info")?;
        self.working_copy_root_path = captures.get(1).unwrap().as_str().to_string();
        self.url = captures.get(2).unwrap().as_str().to_string();
        self.relative_url = captures.get(3).unwrap().as_str().to_string();
        self.repo_root = captures.get(4).unwrap().as_str().to_string();
        self.repo_uuid = captures.get(5).unwrap().as_str().to_string();
        self.revision = captures
            .get(6)
            .unwrap()
            .as_str()
            .to_string()
            .parse()
            .context("Can't convert revision string to number")?;
        self.node_kind = captures.get(7).unwrap().as_str().to_string();
        self.schedule = captures.get(8).unwrap().as_str().to_string();
        self.last_changed_author = captures.get(9).unwrap().as_str().to_string();
        self.last_changed_revision = captures
            .get(10)
            .unwrap()
            .as_str()
            .to_string()
            .parse()
            .context("Can't convert last changed revision to number")?;
        self.last_changed_date = captures.get(11).unwrap().as_str().to_string();

        Ok(())
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
