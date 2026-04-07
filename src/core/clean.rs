use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{Context, bail};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;

use crate::utils::progress_bar::{TICK_CHARS, TICK_INTERVAL};
use crate::utils::{RepoInfo, RepoType, normalize_path};

fn svn_untracked_files(dirs: Vec<&str>) -> anyhow::Result<Vec<PathBuf>> {
    let output = Command::new("svn")
        .arg("status")
        .args(dirs.iter())
        .output()
        .context(format!("Command `svn status` failed"))?;
    if !output.status.success() {
        bail!("Command `svn status {:?}` failed", dirs.join(" "));
    }
    let regex_unversioneds = Regex::new(r#"^\?[[:blank:]]+(.+)[[:blank:]]*$"#)?; // Pattern for out-of-control files
    let output_str = String::from_utf8(output.stdout)?;
    let mut files = Vec::new();
    for line in output_str.lines() {
        if let Some(captures) = regex_unversioneds.captures(line) {
            let item = Path::new(captures.get(1).unwrap().as_str());
            let entry = normalize_path(item);
            files.push(entry);
        }
    }

    Ok(files)
}

fn git_untracked_files(dirs: Vec<&str>) -> anyhow::Result<Vec<PathBuf>> {
    let mut command = Command::new("git");
    command.args(["ls-files", "-oz", "--exclude-standard"]);
    let output = command
        .output()
        .context(format!("Command `svn status` failed"))?;
    if !output.status.success() {
        bail!(
            "Command `git ls-files -oz --exclude-standard {:?}` failed",
            dirs.join(" ")
        );
    }

    let files = String::from_utf8(output.stdout)?
        .split('\0')
        .filter(|&x| !x.is_empty())
        .map(normalize_path)
        .collect::<Vec<PathBuf>>();

    Ok(files)
}

fn untracked_files(
    repo_info: &RepoInfo,
    dirs: Vec<&str>,
) -> anyhow::Result<Vec<PathBuf>> {
    match repo_info.repo_type() {
        RepoType::Git => git_untracked_files(dirs),
        RepoType::Svn => svn_untracked_files(dirs),
    }
}

pub fn clean_build(
    repo_info: &RepoInfo,
    dirs: Option<&Vec<String>>,
    ignore_set: &Vec<Regex>,
) -> anyhow::Result<()> {
    // Check directory
    if env::current_dir()?.as_path() != repo_info.work_dir() {
        bail!(
            r#"Location error! Please run this command under the project root, i.e. "{}"."#,
            repo_info.work_dir()
        );
    }

    let num_steps = 3;
    let mut step: usize = 0;

    // Cleaning the objects generated in building process
    step += 1;
    let pb1 =
        ProgressBar::no_length().with_style(ProgressStyle::with_template(
            &format!("[{}/{}] Removing target objs: {{msg}}", step, num_steps),
        )?);
    let target_dir = normalize_path("target");
    if target_dir.exists() && target_dir.symlink_metadata()?.is_dir() {
        for entry in walkdir::WalkDir::new(&target_dir)
            .contents_first(true)
            .follow_links(false)
        {
            let entry = entry?;

            pb1.set_message(entry.path().to_string_lossy().to_string());
            if entry.file_type().is_dir() {
                fs::remove_dir(entry.path()).context(format!(
                    "Remove {} failed",
                    entry.path().display()
                ))?;
            } else {
                fs::remove_file(entry.path()).context(format!(
                    "Remove {} failed",
                    entry.path().display()
                ))?;
            }
        }
    }
    pb1.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removed target objs.",
        step, num_steps
    ))?);
    pb1.finish();

    // Clean UI files
    step += 1;
    let pb2 =
        ProgressBar::no_length().with_style(ProgressStyle::with_template(
            &format!("[{}/{}] Removing WebUI objs: {{msg}}", step, num_steps),
        )?);
    let webui_dir = normalize_path(repo_info.branch()); // UI directory name is the same as the branch name
    if webui_dir.exists() && webui_dir.symlink_metadata()?.is_dir() {
        for entry in walkdir::WalkDir::new(&webui_dir).contents_first(true) {
            let entry = entry?;

            pb2.set_message(entry.path().to_string_lossy().to_string());
            if entry.file_type().is_dir() {
                fs::remove_dir(entry.path()).context(format!(
                    "Failed to remove {}",
                    entry.path().display()
                ))?;
            } else {
                fs::remove_file(entry.path()).context(format!(
                    "Failed to remove {}",
                    entry.path().display()
                ))?;
            }
        }
    }
    pb2.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removed WebUI objs.",
        step, num_steps
    ))?);
    pb2.finish();

    // Clean unversioned entries
    step += 1;
    let pb3 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] Fetching unversioned files {{spinner}}",
            step, num_steps
        ))?
        .tick_chars(TICK_CHARS),
    );
    pb3.enable_steady_tick(TICK_INTERVAL);

    let dirs: Vec<String> = dirs.map_or(Vec::new(), |x| x.clone());
    let untracked_files = untracked_files(
        &repo_info,
        dirs.iter().map(|x| x.as_str()).collect::<Vec<&str>>(),
    )?;

    pb3.disable_steady_tick();
    pb3.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removing unversioneds: {{msg}}",
        step, num_steps,
    ))?);
    for entry in untracked_files {
        let mut skip = false;
        for re in ignore_set {
            if re.is_match(entry.as_path().to_str().unwrap()) {
                skip = true;
                break;
            }
        }
        if skip {
            continue;
        }
        pb3.set_message(entry.as_path().display().to_string());
        if entry.symlink_metadata()?.is_dir() {
            fs::remove_dir_all(&entry)
                .context(format!("Failed to remove {}", entry.display()))?;
        } else {
            fs::remove_file(&entry)
                .context(format!("Failed to remove {}", entry.display()))?;
        }
    }
    pb3.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removed unversioned files.",
        step, num_steps
    ))?);
    pb3.finish();

    Ok(())
}
