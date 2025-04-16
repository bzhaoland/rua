use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::{env, fs};

use anyhow::{Context, bail};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;

use crate::config::PROJ_RUA_DIR;
use crate::submods::compdb::COMPDB_FILE;
use crate::utils::SvnInfo;
use crate::utils::progress_bar::{TICK_CHARS, TICK_INTERVAL};

fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            std::path::Component::RootDir => {
                normalized.push(component);
            }
            std::path::Component::ParentDir => {
                normalized.pop(); // Go up one directory
            }
            std::path::Component::Normal(name) => {
                normalized.push(name); // Push normal components
            }
            _ => {} // Skip current directory (.) and others
        }
    }
    normalized
}

pub fn clean_build(
    dirs: Option<&Vec<String>>,
    ignores: Option<&Vec<String>>,
) -> anyhow::Result<()> {
    // Check directory
    let svninfo = SvnInfo::new()?;
    let _a = String::from("asdf").as_str();
    if env::current_dir()?.as_path() != svninfo.working_copy_root_path() {
        bail!(
            r#"Location error! Please run this command under the project root, i.e. "{}"."#,
            svninfo.working_copy_root_path().display()
        );
    }

    let mut ignores = ignores
        .map(|x| {
            x.iter()
                .map(|p| normalize_path(Path::new(p)))
                .collect::<Vec<PathBuf>>()
        })
        .unwrap_or_default();
    ignores.push(PathBuf::from_str(PROJ_RUA_DIR)?);
    ignores.push(PathBuf::from_str(".cache")?); // clangd cache
    ignores.push(PathBuf::from_str(COMPDB_FILE)?);

    let num_steps = 3;
    let mut step: usize = 0;

    // Cleaning the objects generated in building process
    step += 1;
    let pb1 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removing target objs: {{msg}}",
        step, num_steps
    ))?);
    let target_dir = normalize_path("target");
    if target_dir.exists() && target_dir.symlink_metadata()?.is_dir() {
        for entry in walkdir::WalkDir::new(&target_dir)
            .contents_first(true)
            .follow_links(false)
        {
            let entry = entry?;
            if ignores.iter().any(|x| entry.path().starts_with(x)) {
                continue;
            }

            pb1.set_message(entry.path().to_string_lossy().to_string());
            if entry.file_type().is_dir() {
                fs::remove_dir(entry.path())
                    .context(format!("Remove {} failed", entry.path().display()))?;
            } else {
                fs::remove_file(entry.path())
                    .context(format!("Remove {} failed", entry.path().display()))?;
            }
        }
    }
    pb1.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removing target objs...{{msg}}",
        step, num_steps
    ))?);
    pb1.finish_with_message("ok");

    // Clean UI files
    step += 1;
    let pb2 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removing WebUI objs: {{msg}}",
        step, num_steps
    ))?);
    let webui_dir = normalize_path(svninfo.branch_name()); // UI directory name is the same as the branch name
    if webui_dir.exists() && webui_dir.symlink_metadata()?.is_dir() {
        for entry in walkdir::WalkDir::new(&webui_dir).contents_first(true) {
            let entry = entry?;
            if ignores.iter().any(|x| entry.path().starts_with(x)) {
                continue;
            }

            pb2.set_message(entry.path().to_string_lossy().to_string());
            if entry.file_type().is_dir() {
                fs::remove_dir(entry.path())
                    .context(format!("Failed to remove {}", entry.path().display()))?;
            } else if entry.path().is_dir() {
                fs::remove_file(entry.path())
                    .context(format!("Failed to remove {}", entry.path().display()))?;
            }
        }
    }
    pb2.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removing WebUI objs...{{msg}}",
        step, num_steps
    ))?);
    pb2.finish_with_message("ok");

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
    let output = Command::new("svn")
        .arg("status")
        .args(dirs.iter())
        .output()
        .context(format!("Command `svn status {:?}` failed", dirs))?;
    if !output.status.success() {
        bail!("Command `svn status {:?}` failed", dirs.join(" "));
    }
    pb3.disable_steady_tick();
    pb3.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removing unversioneds: {{msg}}",
        step, num_steps,
    ))?);
    let regex_unversioneds = Regex::new(r#"^\?[[:blank:]]+(.+)[[:blank:]]*$"#)?; // Pattern for out-of-control files
    let output_str = String::from_utf8(output.stdout)?;
    for line in output_str.lines() {
        if let Some(captures) = regex_unversioneds.captures(line) {
            let item = Path::new(captures.get(1).unwrap().as_str());
            let entry = normalize_path(item);
            if ignores.iter().any(|x| x == entry.as_path()) {
                continue;
            }
            pb3.set_message(entry.as_path().to_string_lossy().to_string());
            if entry.symlink_metadata()?.is_dir() {
                fs::remove_dir_all(&entry)
                    .context(format!("Failed to remove {}", entry.display()))?;
            } else {
                fs::remove_file(&entry).context(format!("Failed to remove {}", entry.display()))?;
            }
        }
    }
    pb3.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Removing unversioneds...{{msg}}",
        step, num_steps
    ))?);
    pb3.finish_with_message("ok");

    Ok(())
}
