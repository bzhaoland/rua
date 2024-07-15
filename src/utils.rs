use std::ffi;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::process::Command;

use anyhow::{Context, Error, Result};
use regex::Regex;

/// Get current machine's hostname
#[allow(dead_code)]
pub fn get_hostname() -> Result<OsString> {
    let hostname_bufsize = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) } as usize;
    let mut hostname_buf = vec![0; hostname_bufsize + 1];
    let retcode = unsafe {
        libc::gethostname(
            hostname_buf.as_mut_ptr() as *mut libc::c_char,
            hostname_buf.len(),
        )
    };
    if retcode != 0 {
        return Err(Error::msg("Failed to get hostname"));
    }

    let end = hostname_buf
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(hostname_buf.len());
    hostname_buf.resize(end, 0);
    Ok(OsString::from_vec(hostname_buf))
}

/// When `svn` utility is available and `svn info` ran successfully
pub fn get_svn_branch() -> Option<String> {
    let output = Command::new("svn").arg("info").output();

    if output.is_err() {
        return None;
    }

    let output = output.unwrap();
    if !output.status.success() {
        return None;
    }
    let output = String::from_utf8(output.stdout).unwrap();

    // Get the full branch name from the svn info
    let branch_pattern = Regex::new(r#"Relative URL: \^/branches/([\w-]+)\n"#)
        .context("Error building regex pattern for capturing branch name")
        .unwrap();
    let branch_name = branch_pattern.captures(&output)?.get(1)?.as_str();

    Some(branch_name.to_string())
}

/// Get login name
#[allow(dead_code)]
pub fn get_login_name() -> String {
    let loginname = unsafe { ffi::CStr::from_ptr(libc::getlogin()) };
    loginname.to_string_lossy().to_string()
}

/// Get current username through external command `id -un`.
/// Unfortunately, crates `whoami` and `users` both function uncorrectly,
/// they got nothing when call corresponding function to get current username.
/// Besides, method using `libc::getuid` + `libc::getpwid` wrapped in an unsafe
/// block functioned uncorrectly too in company's CentOS7 server. Maybe it is
/// because there is no `passwd` table available on the server.
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
