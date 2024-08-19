use std::env;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
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
    hostname_buf.resize(end, 0);
    Ok(OsString::from_vec(hostname_buf))
}

/// Get current username through external command `id -un`.
/// Unfortunately, crates `whoami` and `users` both function uncorrectly,
/// they got nothing when calling corresponding functions to get current username.
/// Besides, method using `libc::getuid` or `libc::getpwid` wrapped in an unsafe
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
        String::from_utf8(output.stdout)
            .unwrap()
            .strip_suffix('\n')
            .unwrap()
            .to_string(),
    )
}

pub struct SvnInfo {
    working_copy_root_path: Option<String>,
    url: Option<String>,
    relative_url: Option<String>,
    repository_root: Option<String>,
    repository_uuid: Option<String>,
    revision: Option<usize>,
    node_kind: Option<String>,
    schedule: Option<String>,
    last_changed_author: Option<String>,
    last_changed_revision: Option<usize>,
    last_changed_date: Option<String>,
}

impl SvnInfo {
    pub fn new() -> anyhow::Result<Self> {
        let mut instance = SvnInfo {
            working_copy_root_path: None,
            url: None,
            relative_url: None,
            repository_root: None,
            repository_uuid: None,
            revision: None,
            node_kind: None,
            schedule: None,
            last_changed_author: None,
            last_changed_revision: None,
            last_changed_date: None,
        };

        instance.info()?;

        Ok(instance)
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
        let pattern = regex::Regex::new(
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

        let captures = pattern
            .captures(&info)
            .context("Error capturing svn info")?;
        self.working_copy_root_path = captures.get(1).map(|x| x.as_str().to_string());
        self.url = captures.get(2).map(|x| x.as_str().to_string());
        self.relative_url = captures.get(3).map(|x| x.as_str().to_string());
        self.repository_root = captures.get(4).map(|x| x.as_str().to_string());
        self.repository_uuid = captures.get(5).map(|x| x.as_str().to_string());
        self.revision = captures.get(6).map(|x| {
            x.as_str()
                .parse()
                .expect("Error parsing as an integer for revision")
        });
        self.node_kind = captures.get(7).map(|x| x.as_str().to_string());
        self.schedule = captures.get(8).map(|x| x.as_str().to_string());
        self.last_changed_author = captures.get(9).map(|x| x.as_str().to_string());
        self.last_changed_revision = captures.get(10).map(|x| {
            x.as_str()
                .parse()
                .expect("Error parsing as an integer for revision")
        });
        self.last_changed_date = captures.get(11).map(|x| x.as_str().to_string());

        Ok(())
    }

    #[allow(dead_code)]
    pub fn working_copy_root_path(&self) -> Option<PathBuf> {
        self.working_copy_root_path
            .as_ref()
            .map(PathBuf::from)
    }

    #[allow(dead_code)]
    pub fn url(&self) -> Option<&String> {
        self.url.as_ref()
    }

    #[allow(dead_code)]
    pub fn relative_url(&self) -> Option<&String> {
        self.relative_url.as_ref()
    }

    #[allow(dead_code)]
    pub fn branch_name(&self) -> Option<String> {
        let rel_url = self.relative_url.as_ref()?;
        let branch_pattern = Regex::new(r#"\^/branches/([-\w]+)"#)
            .context("Error building regex pattern for capturing branch name")
            .unwrap();
        let branch_name = branch_pattern
            .captures(rel_url)
            .expect("Error capturing branch name")
            .get(1)
            .unwrap()
            .as_str();

        Some(branch_name.to_string())
    }

    #[allow(dead_code)]
    pub fn repository_root(&self) -> Option<&String> {
        self.repository_root.as_ref()
    }

    #[allow(dead_code)]
    pub fn repository_uuid(&self) -> Option<&String> {
        self.repository_uuid.as_ref()
    }

    #[allow(dead_code)]
    pub fn revision(&self) -> Option<usize> {
        self.revision
    }

    #[allow(dead_code)]
    pub fn node_kind(&self) -> Option<&String> {
        self.node_kind.as_ref()
    }

    #[allow(dead_code)]
    pub fn schedule(&self) -> Option<&String> {
        self.schedule.as_ref()
    }

    #[allow(dead_code)]
    pub fn last_changed_author(&self) -> Option<&String> {
        self.last_changed_author.as_ref()
    }

    #[allow(dead_code)]
    pub fn last_changed_revision(&self) -> Option<usize> {
        self.last_changed_revision
    }

    #[allow(dead_code)]
    pub fn last_changed_date(&self) -> Option<&String> {
        self.last_changed_date.as_ref()
    }

    #[allow(dead_code)]
    pub fn is_proj_root(&self) -> bool {
        env::current_dir().unwrap() == PathBuf::from(self.working_copy_root_path.as_ref().unwrap())
    }
}
