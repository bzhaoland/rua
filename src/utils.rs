use std::env;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

use anyhow::{bail, Context};
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

/// Check whether is located at project root.
#[allow(dead_code)]
pub fn is_at_proj_root() -> anyhow::Result<bool> {
    // Check location with svn command
    let proj_root = get_proj_root()?;
    if env::current_dir()? != proj_root {
        return anyhow::Ok(false);
    }

    anyhow::Ok(true)
}

/// Get project root path
#[allow(dead_code)]
pub fn get_proj_root() -> anyhow::Result<PathBuf> {
    // Check location with svn command
    let output = Command::new("svn")
        .arg("info")
        .output()
        .context(r#"Command `svn info` failed"#)?;
    if !output.status.success() {
        anyhow::bail!(
            anyhow::anyhow!(String::from_utf8_lossy(&output.stderr).to_string())
                .context(r#"Command `svn info` failed."#)
        );
    }
    let output = String::from_utf8_lossy(&output.stdout);
    let pattern = regex::Regex::new(r#"Working Copy Root Path: (.*)\n"#)?;
    let captures = pattern.captures(&output);
    if captures.is_none() {
        bail!("Can not find the project root path")
    }

    let proj_root = PathBuf::from_str(captures.unwrap().get(1).unwrap().as_str())?;

    anyhow::Ok(proj_root)
}

/// When `svn` utility is available and `svn info` ran successfully
pub fn get_svn_branch() -> anyhow::Result<Option<String>> {
    let output = Command::new("svn")
        .arg("info")
        .output()
        .context(r#"Command `svn info` failed"#)?;
    if !output.status.success() {
        anyhow::bail!(
            anyhow::anyhow!(String::from_utf8_lossy(&output.stderr).to_string())
                .context(r#"Command `svn info` failed."#)
        );
    }
    let output = String::from_utf8_lossy(&output.stdout);

    // Get the full branch name from the svn info
    let branch_pattern = Regex::new(r#"Relative URL: \^/branches/([-\w]+)"#)
        .context("Error building regex pattern for capturing branch name")
        .unwrap();
    let branch_name = branch_pattern
        .captures(&output)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();

    anyhow::Ok(Some(branch_name.to_string()))
}

/// Fetch svn revision
#[allow(dead_code)]
pub fn get_svn_revision() -> anyhow::Result<usize> {
    let output = Command::new("svn")
        .arg("info")
        .output()
        .context(r#"Command `svn info` failed"#)?;
    if !output.status.success() {
        anyhow::bail!(
            anyhow::anyhow!(String::from_utf8_lossy(&output.stderr).to_string())
                .context(r#"Command `svn info` failed."#)
        );
    }
    let output = String::from_utf8_lossy(&output.stdout);

    let pattern_revision = Regex::new(r#"(?im)^Revision:[[:space:]]*([[:digit:]]+)$"#)?;
    let captures = pattern_revision
        .captures(&output)
        .context("Failed to match svn revision")?;
    let revision = captures.get(1).unwrap();

    Ok(revision.as_str().parse().unwrap())
}
