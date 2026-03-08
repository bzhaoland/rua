use std::env::current_dir;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::{ffi::OsString, fmt::Display};

use anyhow::{Context, anyhow, bail};
use gix;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
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
/// inside company's dev env. Besides, methods by wrapping `libc::getuid` or `libc::getpwid`
/// or `libc::getlogin` does not work too on the CentOS7 server in company. Maybe there is no
/// `passwd` table available.
#[allow(dead_code)]
pub fn get_username() -> Option<String> {
    if let Ok(output) = Command::new("id").arg("-un").output()
        && output.status.success()
    {
        Some(
            OsString::from_vec(output.stdout)
                .into_string()
                .unwrap()
                .trim()
                .to_string(),
        )
    } else {
        None
    }
}

#[derive(Clone, Debug)]
pub struct RepoInfo {
    repo_type: RepoType,
    work_dir: String,
    commit_id: String,
    committer: String,
    commit_time: String,
    branch: String,
}

#[derive(Clone, Copy, Debug)]
pub enum RepoType {
    Git,
    Svn,
}

impl RepoInfo {
    pub fn new() -> anyhow::Result<Self> {
        let repo_type =
            Self::detect_repo_type()?.expect("Failed to get repo type");
        let work_dir;
        let commit_id;
        let commit_time;
        let committer;
        let branch;
        match repo_type {
            RepoType::Git => {
                let repo = GitInfo::new()?;
                work_dir = repo.work_dir().to_string();
                commit_id = repo.commit_id().to_string();
                commit_time = repo.commit_time().to_string();
                committer = repo.committer().to_string();
                branch = repo.branch().to_string();
            }
            RepoType::Svn => {
                let repo = SvnInfo::new()?;
                work_dir =
                    repo.working_copy_root_path().to_string_lossy().to_string();
                commit_id = repo.revision().to_string();
                commit_time = repo.last_changed_date().to_string();
                committer = repo.last_changed_author().to_string();
                branch = repo.branch_name().to_string();
            }
        }

        Ok(RepoInfo {
            repo_type,
            work_dir,
            commit_id,
            committer,
            commit_time,
            branch,
        })
    }

    fn detect_repo_type() -> anyhow::Result<Option<RepoType>> {
        let mut current = current_dir().context("Failed to get current dir")?;
        loop {
            if current.join(".git").is_dir() {
                return Ok(Some(RepoType::Git));
            }
            if current.join(".svn").is_dir() {
                return Ok(Some(RepoType::Svn));
            }
            if !current.pop() {
                break;
            }
        }
        Ok(None)
    }

    pub fn repo_type(&self) -> RepoType {
        self.repo_type
    }

    pub fn work_dir(&self) -> &str {
        &self.work_dir
    }

    pub fn commit_id(&self) -> &str {
        &self.commit_id
    }

    pub fn commit_time(&self) -> &str {
        &self.commit_time
    }

    pub fn committer(&self) -> &str {
        &self.committer
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }
}

#[derive(Clone, Debug)]
struct GitInfo {
    work_dir: String,
    commit_id: String,
    committer: String,
    commit_time: String,
    branch: String,
}

impl GitInfo {
    pub(self) fn new() -> anyhow::Result<Self> {
        let repo = gix::discover(".")?;
        let work_dir = repo
            .workdir()
            .expect("Can not find work dir for this git repo");
        let commit = repo
            .head()
            .context("Failed to get head")?
            .id()
            .expect("Failed to get HEAD commit object")
            .object()
            .context("Failed to get the object associated with HEAD")?;
        let commit_id = commit.id;
        let commit = commit.to_commit_ref();
        let committer = commit.committer()?;
        let commit_time = commit.committer()?.time()?;
        let branch = repo
            .head()?
            .referent_name()
            .expect("Failed to get branch name")
            .to_string();
        Ok(GitInfo {
            work_dir: std::fs::canonicalize(work_dir)
                .context("Failed to canonicalize work dir")?
                .to_string_lossy()
                .to_string(),
            commit_id: commit_id.to_string(),
            committer: committer.name.to_string(),
            commit_time: commit_time.to_string(),
            branch: branch,
        })
    }

    pub(self) fn work_dir(&self) -> &str {
        &self.work_dir
    }

    pub(self) fn commit_id(&self) -> &str {
        &self.commit_id
    }

    pub(self) fn committer(&self) -> &str {
        &self.committer
    }

    pub(self) fn commit_time(&self) -> &str {
        &self.commit_time
    }

    pub(self) fn branch(&self) -> &str {
        &self.branch
    }
}

#[derive(Clone, Debug)]
pub struct SvnInfo {
    working_copy_root_path: String,
    url: String,
    relative_url: String,
    repo_root: String,
    repo_uuid: String,
    revision: i64,
    path: String,
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
path: {}
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
            self.path,
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
            .arg("--xml")
            .output()
            .context("Command `svn info` failed")?;
        if !result.status.success() {
            bail!(
                anyhow!(String::from_utf8_lossy(&result.stderr).to_string())
                    .context("Command `svn info` failed")
            );
        }
        let output = str::from_utf8(result.stdout.as_slice())
            .context("Not valid UTF-8 string")?;
        let mut reader = Reader::from_str(output);
        let mut kind = None;
        let mut path = None;
        let mut revision = None;
        let mut url = None;
        let mut rel_url = None;
        let mut repo_root = None;
        let mut repo_uuid = None;
        let mut workcopy_root = None;
        let mut workcopy_schedule = None;
        let mut _workcopy_depth = None;
        let mut commit_revision = None;
        let mut commit_author = None;
        let mut commit_date = None;
        let mut level: Vec<u8> = Vec::with_capacity(1024);
        loop {
            match reader.read_event() {
                Err(_) => {
                    bail!(anyhow!(
                        "Error at position {}",
                        reader.error_position()
                    ));
                }
                Ok(Event::Start(elem)) => {
                    let tagname = elem.name().as_ref().to_vec();
                    level.push(b'/');
                    level.extend_from_slice(tagname.as_slice());
                    match level.as_slice() {
                        b"/info/entry" => {
                            kind = Some(String::from_utf8(
                                elem.try_get_attribute("kind")?
                                    .expect("Failed to get kind attribute of /info/entry node")
                                    .value
                                    .to_vec(),
                            )?);
                            path = Some(String::from_utf8(
                                elem.try_get_attribute("path")?
                                    .expect("Failed to get path attribute of /info/entry node")
                                    .value
                                    .to_vec(),
                            )?);
                            revision = Some(String::from_utf8(
                                elem.try_get_attribute("revision")?
                                    .expect("Failed to get revision attribute of /info/entry node")
                                    .value
                                    .to_vec(),
                            )?);
                        }
                        b"/info/entry/commit" => {
                            commit_revision = Some(String::from_utf8(
                                elem.try_get_attribute("revision")?
                                    .expect(
                                        "Failed to get revision attribute of /info/entry/commit node",
                                    )
                                    .value
                                    .to_vec(),
                            )?);
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(elem)) => {
                    if level.ends_with(elem.name().as_ref()) {
                        level.truncate(
                            level.len() - elem.name().as_ref().len() - 1,
                        );
                    }
                }
                Ok(Event::Text(elem)) => {
                    let s = elem.decode()?.trim().to_owned();
                    match level.as_slice() {
                        b"/info/entry/url" => {
                            url = Some(s);
                        }
                        b"/info/entry/relative-url" => {
                            rel_url = Some(s);
                        }
                        b"/info/entry/repository/root" => {
                            repo_root = Some(s);
                        }
                        b"/info/entry/repository/uuid" => {
                            repo_uuid = Some(s);
                        }
                        b"/info/entry/wc-info/wcroot-abspath" => {
                            workcopy_root = Some(s);
                        }
                        b"/info/entry/wc-info/schedule" => {
                            workcopy_schedule = Some(s);
                        }
                        b"/info/entry/wc-info/depth" => {
                            _workcopy_depth = Some(s);
                        }
                        b"/info/entry/commit/author" => {
                            commit_author = Some(s);
                        }
                        b"/info/entry/commit/date" => {
                            commit_date = Some(s);
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }

        Ok(SvnInfo {
            working_copy_root_path: workcopy_root
                .expect("Working copy root path not found"),
            url: url.expect("Url not found"),
            relative_url: rel_url.expect("Relative url not found"),
            repo_root: repo_root.expect("Repository root not found"),
            repo_uuid: repo_uuid.expect("Repository UUID not found"),
            revision: revision
                .expect("Revision not found")
                .parse()
                .context("Revision parse failed")?,
            path: path.expect("Path not found"),
            node_kind: kind.expect("Node kind not found"),
            schedule: workcopy_schedule.expect("Schedule not found"),
            last_changed_author: commit_author
                .expect("Last changed author not found"),
            last_changed_revision: commit_revision
                .expect("Last changed rev not found")
                .parse()
                .context("Last changed rev parse failed")?,
            last_changed_date: commit_date
                .expect("Last changed date not found"),
        })
    }

    #[allow(dead_code)]
    pub fn working_copy_root_path(&self) -> &Path {
        Path::new(self.working_copy_root_path.as_str())
    }

    #[allow(dead_code)]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[allow(dead_code)]
    pub fn relative_url(&self) -> &str {
        &self.relative_url
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
