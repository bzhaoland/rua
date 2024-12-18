use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{anyhow, bail, Context};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::time::Duration;

use crate::utils::SvnInfo;

fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            std::path::Component::RootDir => {
                normalized.push(component);
            }
            std::path::Component::CurDir => {
                // Skip current directory (.)
            }
            std::path::Component::ParentDir => {
                normalized.pop(); // Go up one directory
            }
            std::path::Component::Normal(name) => {
                normalized.push(name); // Push normal components
            }
            _ => {}
        }
    }

    normalized
}

pub fn clean_build(
    dirs: Option<&Vec<String>>,
    ignores: Option<&Vec<String>>,
) -> anyhow::Result<()> {
    // Check directory
    let svn_info = SvnInfo::new()?;
    if env::current_dir()?.as_path() != svn_info.working_copy_root_path() {
        bail!(
            r#"Location error! Please run this command under the project root, i.e. "{}"."#,
            svn_info.working_copy_root_path().display()
        );
    }

    let ignores = ignores
        .map(|x| {
            x.iter()
                .map(|p| normalize_path(Path::new(p)))
                .collect::<Vec<PathBuf>>()
        })
        .unwrap_or_default();

    const TICK_INTERVAL: Duration = Duration::from_millis(200);
    const TICK_CHARS: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

    let num_steps = 3;
    let mut step: usize = 0;

    // Cleaning the objects generated in building process
    step += 1;
    let pb1 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING TARGET OBJS: {{msg:.green}}",
        step, num_steps
    ))?);
    let target_dir = normalize_path("target");
    if target_dir.is_dir() {
        for item in walkdir::WalkDir::new(&target_dir)
            .contents_first(true)
            .into_iter()
        {
            if let Ok(entry) = item {
                if ignores.iter().any(|x| entry.path().starts_with(x)) {
                    continue;
                }

                let path_ = entry.path();
                pb1.set_message(path_.to_string_lossy().to_string());
                if path_.is_file() || path_.is_symlink() {
                    fs::remove_file(path_)
                        .context(format!("Failed to remove file {}", path_.display()))?;
                } else if path_.is_dir() {
                    fs::remove_dir_all(path_)
                        .context(format!("Failed to remove directory {}", path_.display()))?;
                }
            }
        }
    }
    pb1.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING TARGET OBJS...{{msg:.green}}",
        step, num_steps
    ))?);
    pb1.finish_with_message("OK");

    // Clean UI files
    step += 1;
    let pb2 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING WEBUI OBJS: {{msg:.green}}",
        step, num_steps
    ))?);
    let webui_dir = normalize_path(svn_info.branch_name()); // UI directory name is the same as the branch name
    if webui_dir.is_dir() {
        for item in walkdir::WalkDir::new(&webui_dir)
            .contents_first(true)
            .into_iter()
        {
            if let Ok(entry) = item {
                if ignores.iter().any(|x| entry.path().starts_with(x)) {
                    continue;
                }

                let path_ = entry.path();
                pb2.set_message(path_.to_string_lossy().to_string());
                if path_.is_file() || path_.is_symlink() {
                    fs::remove_file(path_)
                        .context(format!("Failed to remove file: {}", path_.display()))?;
                } else if path_.is_dir() {
                    fs::remove_dir_all(path_)
                        .context(format!("Failed to remove directory: {}", path_.display()))?;
                }
            }
        }
    }
    pb2.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING WEBUI OBJS...{{msg:.green}}",
        step, num_steps
    ))?);
    pb2.finish_with_message("OK");

    // Clean unversioned entries
    step += 1;
    let pb3 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] LISTING UNVERSIONEDS {{spinner:.green}}",
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
        bail!("Can't invoke `svn status {:?}`", dirs.join(" "));
    }
    pb3.disable_steady_tick();
    pb3.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING UNVERSIONEDS: {{msg:.green}}",
        step, num_steps,
    ))?);
    let pattern_for_unversioneds = Regex::new(r#"^\?[[:blank:]]+(.+)[[:blank:]]*$"#)
        .context("Failed to construct pattern for unversioned files")?; // Pattern for out-of-control files
    let output_str =
        String::from_utf8(output.stdout).context(anyhow!("Can't convert to String"))?;
    for line in output_str.lines() {
        if let Some(captures) = pattern_for_unversioneds.captures(line) {
            let item = Path::new(captures.get(1).unwrap().as_str());
            let entry = normalize_path(item);
            if ignores.iter().all(|x| x != &entry) {
                pb3.set_message(entry.as_path().to_string_lossy().to_string());
                if entry.is_file() || entry.is_symlink() {
                    fs::remove_file(&entry)
                        .context(format!("Failed to remove: file {}", entry.display()))?;
                } else if entry.is_dir() {
                    fs::remove_dir_all(&entry)
                        .context(format!("Failed to remove: {}", entry.display()))?;
                }
            }
        }
    }
    pb3.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING UNVERSIONEDS...{{msg:.green}}",
        step, num_steps
    ))?);
    pb3.finish_with_message("OK");

    Ok(())
}
