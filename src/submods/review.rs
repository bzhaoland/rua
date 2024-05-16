use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::process::Command;

use anyhow::{Error, Result};
use libc;
use regex::Regex;
use reqwest::Client;

/// Get current machine's hostname
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

pub async fn review(
    bug_id: u32,
    review_id: Option<u32>,
    file_list: &Option<Vec<String>>,
    diff_file: &Option<String>,
    reviewers: &Option<Vec<String>>,
    branch_name: &Option<String>,
    repo_name: &Option<String>,
    revision: &Option<String>,
) -> Result<()> {
    // Get bug class via http request. If the bug class is CustomerIssue,
    // then reject this review request.
    let client = Client::new();
    let bug_class = client
        .get(format!(
            r#"http://10.100.1.150/api/bugz_new.php?type=get_bugclass&bug_id={bug_id}"#
        ))
        .send()
        .await?
        .text()
        .await?;
    if bug_class == "CustomerIssues" {
        return Err(Error::msg("CustomerIssues cannot be reviewed"));
    }

    // Get branch name
    let branch_name = match branch_name {
        Some(v) => v.to_owned(),
        None => {
            let cmdres = Command::new("svn").arg("info").output()?;
            if !cmdres.status.success() {
                return Err(Error::msg("Failed to get svn info"));
            }

            let output = String::from_utf8_lossy(&cmdres.stdout).to_string();
            let branch_pattern =
                Regex::new(r#"Relative URL: \^/(?:(?:tags|branches)/([\w-]+)|(trunk))"#).unwrap();
            let caps = branch_pattern
                .captures(&output)
                .expect("Failed to capture branch name");
            caps.get(1).unwrap().as_str().to_owned()
        }
    };

    // Get modified files
    let cmdres = Command::new("svn").args(["status", "-q"]).output()?;
    if !cmdres.status.success() {
        return Err(Error::msg("Failed to execute `svn status -q`"));
    }

    let mut comm = Command::new("python2");
    comm.arg("/usr/lib/python2.7/site-packages/RBTools-0.4.1-py2.7.egg/rbtools/postreview-cops.py")
        .args([
            "--summary",
            format!("Code review for bug {bug_id}").as_str(),
        ])
        .arg(format!("--bugs-closed={bug_id}"))
        .arg(format!("--branch={branch_name}"))
        .arg("--server=http://cops-server.hillstonedev.com:8181")
        .arg("-p");

    // If review id is not given, then start a new one
    match review_id {
        Some(v) => comm.args(["-r", v.to_string().as_str()]),
        None => comm.arg(r#"--description-file=/devel/sw/bin/review_template"#),
    };

    if file_list.is_some() {
        comm.args(file_list.to_owned().unwrap());
    }

    let status = comm.status()?;
    if !status.success() {
        return Err(Error::msg("Failed to execute postreview-cops.py"));
    }

    Ok(())
}
