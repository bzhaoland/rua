use std::process::Command;

use anyhow::{Error, Result};
use regex::Regex;
use reqwest::Client;

pub struct ReviewOptions {
    pub bug_id: u32,
    pub review_id: Option<u32>,
    pub file_list: Option<Vec<String>>,
    pub diff_file: Option<String>,
    pub reviewers: Option<Vec<String>>,
    pub branch_name: Option<String>,
    pub repo_name: Option<String>,
    pub revisions: Option<String>,
}

pub async fn review(options: &ReviewOptions) -> Result<()> {
    // Make a http request and get the response. The response text indicates
    // the category of the bug.
    let client = Client::new();
    let bug_class = client
        .get(format!(
            r#"http://10.100.1.150/api/bugz_new.php?type=get_bugclass&bug_id={}"#,
            options.bug_id
        ))
        .send()
        .await?
        .text()
        .await?;
    if bug_class == "CustomerIssues" {
        return Err(Error::msg("CustomerIssues cannot be reviewed"));
    }

    // Get branch name from the output of svn info
    let branch_name = match options.branch_name.as_ref() {
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

    // Get files to be commited from the output of svn status.
    let cmdres = Command::new("svn").args(["status", "-q"]).output()?;
    if !cmdres.status.success() {
        return Err(Error::msg("Failed to execute `svn status -q`"));
    }

    let mut comm = Command::new("python2");
    comm.arg("/usr/lib/python2.7/site-packages/RBTools-0.4.1-py2.7.egg/rbtools/postreview-cops.py")
        .arg(format!("--summary=Code review for bug {}", options.bug_id))
        .arg(format!("--bugs-closed={}", options.bug_id))
        .arg(format!("--branch={}", branch_name))
        .arg("--server=http://cops-server.hillstonedev.com:8181")
        .arg("-p"); // Publish it immediately

    // If review id is not given, then start a new one
    match options.review_id {
        Some(v) => comm.args(["-r", v.to_string().as_str()]),
        None => comm.arg(r#"--description-file=/devel/sw/bin/review_template"#),
    };

    if options.file_list.is_some() {
        comm.args(options.file_list.as_ref().unwrap());
    }

    let status = comm.status()?;
    if !status.success() {
        return Err(Error::msg("Failed to execute postreview-cops.py"));
    }

    Ok(())
}
