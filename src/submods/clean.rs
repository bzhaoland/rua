use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use anstyle::{AnsiColor, Color, Style};
use anyhow::{anyhow, bail, Context};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::utils::SvnInfo;

const COLOR_ANSI_GRN: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));

fn trucate_string(s: &str, l: usize) -> String {
    if s.chars().count() <= l {
        s.to_owned()
    } else {
        s.chars().skip(s.chars().count() - l).collect()
    }
}

pub fn clean_build(
    dirs: Option<Vec<OsString>>,
    ignores: Option<&Vec<OsString>>
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
                .filter_map(|p| fs::canonicalize(p).ok())
                .collect::<Vec<PathBuf>>()
        })
        .unwrap_or_default();

    const REFRESH_INTERVAL: u64 = 200; // In milliseconds
    const DISPLAY_PATH_LEN: usize = 32;

    let num_steps = 3;
    let mut step: usize = 0;

    // Cleaning the objects generated in building process
    step += 1;
    let pb1 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING TARGET OBJS: {{msg:.green}}",
        step, num_steps
    ))?);
    let target_dir = fs::canonicalize("target");
    if target_dir.is_ok() {
        for x in walkdir::WalkDir::new(target_dir.unwrap())
            .contents_first(true)
            .into_iter()
            .filter(|x| {
                if x.is_err() {
                    return false;
                }
                let x = x.as_ref().unwrap();
                let entry = x.path();

                ignores.iter().all(|x| x.as_path() != entry)
            })
        {
            let entry = x?;
            let path_ = entry.path();
            let file_indicator = trucate_string(&path_.to_string_lossy(), DISPLAY_PATH_LEN);
            pb1.set_message(file_indicator);

            if path_.is_file() || path_.is_symlink() {
                fs::remove_file(path_)
                    .context(format!("Error removing file {}", path_.display()))?;
            } else if path_.is_dir() {
                fs::remove_dir_all(path_)
                    .context(format!("Error removing directory {}", path_.display()))?;
            }
        }
    }
    pb1.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING TARGET OBJS...{}OK{:#}",
        step, num_steps, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    ))?);
    pb1.finish();

    // Clean unversioned entries
    step += 1;
    let pb2 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] LISTING UNVERSIONEDS {{spinner:.green}}",
        step, num_steps
    ))?);
    pb2.enable_steady_tick(Duration::from_millis(REFRESH_INTERVAL));
    let dirs: Vec<OsString> = dirs.unwrap_or_default();
    let output = Command::new("svn")
        .arg("status")
        .args(dirs.iter())
        .output()
        .context(format!("Command `svn status {:?}` failed", dirs))?;
    if !output.status.success() {
        bail!(
            "Error invoking `svn status {:?}`",
            dirs.iter()
                .map(|x| x.to_string_lossy().to_string().to_owned())
                .collect::<Vec<String>>()
                .join(" ")
        );
    }
    let pattern_for_unversioneds =
        regex::Regex::new(r#"(?m)^\?[[:blank:]]+([^[:blank:]].*?)[[:space:]]*$"#)
            .context("Error creating regex pattern")?; // Pattern for out-of-control files
    let output_str =
        String::from_utf8(output.stdout).context(anyhow!("Error converting to `String` type"))?;
    let unversioned_files = pattern_for_unversioneds
        .captures_iter(&output_str)
        .filter_map(|c| fs::canonicalize((c.extract::<1>().1)[0]).ok())
        .filter(|x| ignores.iter().all(|i| x != i))
        .collect::<Vec<PathBuf>>();
    pb2.disable_steady_tick();
    pb2.set_length(unversioned_files.len() as u64);
    pb2.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING UNVERSIONEDS: {{msg:.green}}",
        step, num_steps,
    ))?);
    for item in unversioned_files.iter() {
        pb2.set_message(item.as_path().to_string_lossy().to_string());
        if item.is_file() || item.is_symlink() {
            fs::remove_file(item).context(format!("Error removing file {}", item.display()))?;
        } else if item.is_dir() {
            fs::remove_dir_all(item)
                .context(format!("Error removing directory {}", item.display()))?;
        }
    }
    pb2.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING UNVERSIONEDS...{}OK{:#}",
        step, num_steps, COLOR_ANSI_GRN, COLOR_ANSI_GRN,
    ))?);
    pb2.finish();

    // Clean UI files
    step += 1;
    let pb3 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING WEBUI OBJS: {{msg:.green}}",
        step, num_steps
    ))?);
    let webui_dir = fs::canonicalize(svn_info.branch_name()); // UI directory name is the same as the branch name
    if webui_dir.is_ok() {
        for x in walkdir::WalkDir::new(webui_dir.unwrap())
            .contents_first(true)
            .into_iter()
            .filter(|x| {
                if x.is_err() {
                    return false;
                }
                let x = x.as_ref().unwrap();
                let entry = x.path();
                ignores.iter().all(|i| i.as_path() != entry)
            })
        {
            let entry = x?;
            let path_ = entry.path();
            pb3.set_message(trucate_string(&path_.to_string_lossy(), DISPLAY_PATH_LEN));
            if path_.is_file() || path_.is_symlink() {
                fs::remove_file(path_)
                    .context(format!("Error removing file {}", path_.display()))?;
            } else if path_.is_dir() {
                fs::remove_dir_all(path_)
                    .context(format!("Error removing directory {}", path_.display()))?;
            }
        }
    }
    pb3.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] CLEANING UI OBJS...{}OK{:#}",
        step, num_steps, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    ))?);
    pb3.finish();

    Ok(())
}
