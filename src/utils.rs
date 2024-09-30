use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Context};
use regex::Regex;

/// Get current machine's hostname
#[allow(dead_code)]
pub fn get_hostname() -> anyhow::Result<OsString> {
    let hostname_bufsize = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) } as usize;
    let mut hostname_buf = vec![0; hostname_bufsize + 1];
    let retcode = unsafe {
        libc::gethostname(
            hostname_buf.as_mut_ptr() as *mut libc::c_char,
            hostname_buf.len(),
        )
    };
    if retcode != 0 {
        anyhow::bail!("Get hostname failed");
    }

    let end = hostname_buf
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(hostname_buf.len());
    hostname_buf.truncate(end);
    Ok(OsString::from_vec(hostname_buf))
}

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
    repository_root: String,
    repository_uuid: String,
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
            repository_root: String::new(),
            repository_uuid: String::new(),
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
        self.repository_root.clear();
        self.repository_uuid.clear();
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
        .expect("Error building regex pattern for project root");

        let captures = pattern.captures(&info).context("Error matching svn info")?;
        self.working_copy_root_path = captures
            .get(1)
            .context("Error capturing the working copy root path")?
            .as_str()
            .to_string();
        self.url = captures
            .get(2)
            .context("Error capturing the url")?
            .as_str()
            .to_string();
        self.relative_url = captures
            .get(3)
            .context("Error capturing the relative url")?
            .as_str()
            .to_string();
        self.repository_root = captures
            .get(4)
            .context("Error capturing the repository root")?
            .as_str()
            .to_string();
        self.repository_uuid = captures
            .get(5)
            .context("Error capturing the repository uuid")?
            .as_str()
            .to_string();
        self.revision = captures
            .get(6)
            .context("Error capturing the revision")?
            .as_str()
            .to_string()
            .parse()
            .context("Error parsing the revision part into an integer")?;
        self.node_kind = captures
            .get(7)
            .context("Error capturing the node kind")?
            .as_str()
            .to_string();
        self.schedule = captures
            .get(8)
            .context("Error capturing the schedule")?
            .as_str()
            .to_string();
        self.last_changed_author = captures
            .get(9)
            .context("Error capturing the last changed author")?
            .as_str()
            .to_string();
        self.last_changed_revision = captures
            .get(10)
            .context("Error capturing the last changed revision")?
            .as_str()
            .to_string()
            .parse()
            .context("Error parsing the last changed revision part into an integer")?;
        self.last_changed_date = captures
            .get(11)
            .context("Error capturing the last changed date")?
            .as_str()
            .to_string();

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
            .context("Error building regex pattern for capturing branch name")
            .unwrap();
        let branch_name = branch_pattern
            .captures(rel_url)
            .expect("Error capturing branch name")
            .get(1)
            .unwrap()
            .as_str();
        branch_name
    }

    #[allow(dead_code)]
    pub fn repository_root(&self) -> &str {
        self.repository_root.as_str()
    }

    #[allow(dead_code)]
    pub fn repository_uuid(&self) -> &str {
        self.repository_uuid.as_str()
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
