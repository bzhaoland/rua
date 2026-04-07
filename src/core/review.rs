use std::{path::Path, process::Command};

use anyhow::{Context, bail};
use reqwest::Client;

use crate::utils::{RepoInfo, RepoType};

#[allow(dead_code)]
pub struct ReviewOptions {
    pub repo_type: RepoType,
    pub bug_id: u32,
    pub review_id: Option<u32>,
    pub files: Option<Vec<String>>,
    pub diff_file: Option<String>,
    pub reviewers: Option<Vec<String>>,
    pub branch_name: Option<String>,
    pub repo_name: Option<String>,
    pub revisions: Option<String>,
    pub template_file: Option<String>,
}

pub async fn review(options: &ReviewOptions) -> anyhow::Result<()> {
    const DEFAULT_REVIEW_TEMPLATE_4SVN: &str = "/devel/sw/bin/review_template";
    const DEFAULT_REVIEW_TEMPLATE_4GIT: &str =
        "/devel/sw/buildserver_gitcops/review_template";

    let default_review_template = match options.repo_type {
        RepoType::Svn => DEFAULT_REVIEW_TEMPLATE_4SVN,
        RepoType::Git => DEFAULT_REVIEW_TEMPLATE_4GIT,
    };

    let review_template_file = options
        .template_file
        .as_deref()
        .unwrap_or(default_review_template);

    // Check for file existence
    if !Path::new(review_template_file).is_file() {
        bail!("File not found: {}", review_template_file)
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
        bail!("Can not review CustomerIssues");
    }

    // Get branch name from the output of svn info
    let branch_name = match options.branch_name.as_ref() {
        Some(v) => v.to_owned(),
        None => RepoInfo::new()?.branch().to_string(),
    };

    // let mut comm = Command::new("python2");
    // comm.args([
    //     "/usr/lib/python2.7/site-packages/RBTools-0.4.1-py2.7.egg/rbtools/postreview-cops.py",
    //     &format!("--summary=Code review for bug {}", options.bug_id),
    //     &format!("--bugs-closed={}", options.bug_id),
    //     &format!("--branch={}", branch_name),
    //     "--server=http://cops-server.hillstonedev.com:8181",
    //     "-p", // Publish it immediately
    // ]);
    let mut comm = Command::new("python3");
    comm.args([
        "/devel/sw/buildserver_gitcops/RBTools-0.4.1/postreview-cops.py",
        &format!("--summary=Code review for bug {}", options.bug_id),
        &format!("--bugs-closed={}", options.bug_id),
        &format!("--branch={}", branch_name),
        "--server=http://cops-server.hillstonedev.com:8181",
        "-p", // Publish it immediately
        "--username=newreview",
        "--password=hillstone",
    ]);

    // If review id is not given, launch a new one
    if let Some(id) = options.review_id {
        comm.args(["-r", &id.to_string()]);
    } else {
        comm.args(["--description-file", review_template_file]);
    }

    let status = if let Some(diff_file) = options.diff_file.as_ref() {
        comm.args(["--diff-filename", diff_file]).status()?
    } else {
        let diff = match options.repo_type {
            RepoType::Git => {
                let mut diff_comm = Command::new("git");
                diff_comm.args(["diff", "--staged"]);
                if options.files.is_some() {
                    diff_comm.args(options.files.as_ref().unwrap());
                }
                let output = diff_comm
                    .output()
                    .context("Failed to get staged changes from git")?;
                if !output.status.success() {
                    bail!(
                        "Command `git diff --staged {:?}` failed",
                        options.files.as_ref().unwrap().join(" ")
                    );
                }
                output.stdout
            }
            RepoType::Svn => {
                let mut diff_comm = Command::new("svn");
                diff_comm.args(["diff"]);
                if options.files.is_some() {
                    diff_comm.args(options.files.as_ref().unwrap());
                }
                let output = diff_comm
                    .output()
                    .context("Failed to get staged changes from git")?;
                if !output.status.success() {
                    bail!(
                        "Command `git diff --staged {:?}` failed",
                        options.files.as_ref().unwrap().join(" ")
                    );
                }
                output.stdout
            }
        };

        let mut child = comm
            .args(["--diff-filename", "-"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn postreview-cops.py")?;
        {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .context("Failed to open stdin")?
                .write_all(&diff)?;
        }
        child.wait()?
    };
    if !status.success() {
        bail!(
            "Run postreview-cops.py failed: {}",
            status.code().context("Aborted")?
        );
    }

    Ok(())
}
