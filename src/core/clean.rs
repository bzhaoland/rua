use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{Context, bail};
use globset::GlobSet;
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
    command.args(["ls-files", "-oz"]);
    let output = command.args(dirs.iter()).output().context(format!(
        "Command `git ls-files -oz {}` failed",
        dirs.join(" ")
    ))?;
    if !output.status.success() {
        bail!("Command `git ls-files -oz {:?}` failed", dirs.join(" "));
    }

    let files = String::from_utf8(output.stdout)?
        .split('\0')
        .filter(|&x| !x.is_empty())
        .map(normalize_path)
        .collect::<Vec<PathBuf>>();

    Ok(files)
}

fn get_untracked_files(repo_info: &RepoInfo, dirs: Vec<&str>) -> anyhow::Result<Vec<PathBuf>> {
    match repo_info.repo_type() {
        RepoType::Git => git_untracked_files(dirs),
        RepoType::Svn => svn_untracked_files(dirs),
    }
}

pub fn clean_build(
    repo_info: &RepoInfo,
    dirs: Option<&Vec<String>>,
    ignore_set: &GlobSet,
) -> anyhow::Result<()> {
    // Check directory
    if env::current_dir()?.as_path() != repo_info.work_dir() {
        bail!(
            r#"Location error! Please run this command under the project root, i.e. "{}"."#,
            repo_info.work_dir()
        );
    }

    // Cleaning the objects generated in building process
    let target_dir = normalize_path("target");
    if target_dir.exists() && target_dir.symlink_metadata()?.is_dir() {
        for entry in walkdir::WalkDir::new(&target_dir)
            .contents_first(true)
            .follow_links(false)
        {
            let entry = entry?;

            println!("Deleting {}", entry.path().display());
            if entry.file_type().is_dir() {
                fs::remove_dir(entry.path())
                    .context(format!("Remove {} failed", entry.path().display()))?;
            } else {
                fs::remove_file(entry.path())
                    .context(format!("Remove {} failed", entry.path().display()))?;
            }
        }
    }

    let webui_dir = normalize_path(repo_info.branch()); // UI directory name is the same as the branch name
    if webui_dir.exists() && webui_dir.symlink_metadata()?.is_dir() {
        for entry in walkdir::WalkDir::new(&webui_dir).contents_first(true) {
            let entry = entry?;

            println!("Deleting {}", entry.path().display());
            if entry.file_type().is_dir() {
                fs::remove_dir(entry.path())
                    .context(format!("Failed to remove {}", entry.path().display()))?;
            } else {
                fs::remove_file(entry.path())
                    .context(format!("Failed to remove {}", entry.path().display()))?;
            }
        }
    }

    // Clean untracked files
    let pb = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!("Fetching untracked files {{spinner}}"))?
            .tick_chars(TICK_CHARS),
    );
    pb.enable_steady_tick(TICK_INTERVAL);
    let dirs: Vec<String> = dirs.map_or(Vec::new(), |x| x.clone());
    let untracked_files = get_untracked_files(
        &repo_info,
        dirs.iter().map(|x| x.as_str()).collect::<Vec<&str>>(),
    )?;
    pb.finish_and_clear();
    for entry in untracked_files {
        if ignore_set.is_match(
            entry
                .as_path()
                .to_str()
                .expect(format!("Failed to convert {:?} to str", entry).as_str()),
        ) {
            continue;
        }
        println!("Deleting {}", entry.display());
        if entry.symlink_metadata()?.is_dir() {
            fs::remove_dir_all(&entry).context(format!("Failed to remove {}", entry.display()))?;
        } else {
            fs::remove_file(&entry).context(format!("Failed to remove {}", entry.display()))?;
        }
    }

    Ok(())
}
