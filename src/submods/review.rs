use std::{path::Path, process::Command};

use anyhow::{bail, Context};
use reqwest::Client;

use crate::utils::SvnInfo;

#[allow(dead_code)]
pub struct ReviewOptions {
    pub bug_id: u32,
    pub review_id: Option<u32>,
    pub files: Option<Vec<String>>,
    pub diff_file: Option<String>,
    pub reviewers: Option<Vec<String>>,
    pub branch_name: Option<String>,
    pub repo_name: Option<String>,
    pub revisions: Option<String>,
    pub description_template_file: Option<String>,
}

pub async fn review(options: &ReviewOptions) -> anyhow::Result<()> {
    const DEFAULT_DESCRIPTION_TEMPLATE_FILE: &str = "/devel/sw/bin/review_template";

    // Check file existence
    let description_template_file = Path::new(DEFAULT_DESCRIPTION_TEMPLATE_FILE);
    if !description_template_file.is_file() {
        bail!("File not found: {}", DEFAULT_DESCRIPTION_TEMPLATE_FILE)
    }

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
        bail!("CustomerIssues cannot be reviewed");
    }

    // Get branch name from the output of svn info
    let branch_name = match options.branch_name.as_ref() {
        Some(v) => v.to_owned(),
        None => SvnInfo::new()?.branch_name().to_string(),
    };

    // Files to commit
    let cmdres = Command::new("svn").args(["status", "-q"]).output()?;
    if !cmdres.status.success() {
        bail!("Command `svn status -q` failed");
    }

    let mut comm = Command::new("python2");
    comm.args([
        "/usr/lib/python2.7/site-packages/RBTools-0.4.1-py2.7.egg/rbtools/postreview-cops.py",
        &format!("--summary=Code review for bug {}", options.bug_id),
        &format!("--bugs-closed={}", options.bug_id),
        &format!("--branch={}", branch_name),
        "--server=http://cops-server.hillstonedev.com:8181",
        "-p", // Publish it immediately
    ]);

    // If review id is not given, then start a new one
    match options.review_id {
        Some(v) => comm.args(["-r", &v.to_string()]),
        None => comm.arg(format!(
            r#"--description-file={}"#,
            options
                .description_template_file
                .as_deref()
                .unwrap_or(DEFAULT_DESCRIPTION_TEMPLATE_FILE)
        )),
    };

    if options.files.is_some() {
        comm.args(options.files.as_ref().unwrap());
    }

    let status = comm.status()?;
    if !status.success() {
        bail!(
            "Run postreview-cops.py failed: {}",
            status.code().context("Aborted")?
        );
    }

    Ok(())
}
